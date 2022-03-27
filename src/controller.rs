use std::hash::Hash;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use uuid::Uuid;
use poem_openapi::Object;
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::time::Instant;

use crate::config::{BucketConfig, ImageKind};
use crate::pipelines::{PipelineController, ProcessingMode, StoreEntry};
use crate::storage::template::StorageBackend;

static BUCKETS: OnceCell<hashbrown::HashMap<u32, BucketController>> = OnceCell::new();

pub fn init_buckets(buckets: hashbrown::HashMap<u32, BucketController>) {
    let _ = BUCKETS.set(buckets);
}

pub fn get_bucket_by_id(bucket_id: u32) -> Option<&'static BucketController> {
    BUCKETS.get_or_init(hashbrown::HashMap::new).get(&bucket_id)
}

pub fn get_bucket_by_name(bucket: impl Hash) -> Option<&'static BucketController> {
    let bucket_id = crate::utils::crc_hash(bucket);
    get_bucket_by_id(bucket_id)
}

async fn get_optional_permit<'a>(
    global: &'a Option<Arc<Semaphore>>,
    local: &'a Option<Semaphore>,
) -> anyhow::Result<Option<SemaphorePermit<'a>>> {
    if let Some(limiter) = global {
        return Ok(Some(limiter.acquire().await?))
    }

    if let Some(limiter) = local {
        return Ok(Some(limiter.acquire().await?))
    }

    Ok(None)
}

#[derive(Object, Debug)]
pub struct UploadInfo {
    /// The generated ID for the file.
    ///
    /// This can be used to access the file for the given bucket.
    image_id: Uuid,

    /// The time spent processing the image in seconds.
    processing_time: f32,

    /// The crc32 checksum of the uploaded image.
    checksum: u32,
}

pub struct BucketController {
    bucket_id: u32,
    global_limiter: Option<Arc<Semaphore>>,
    config: BucketConfig,
    pipeline: PipelineController,
    storage: Arc<dyn StorageBackend>,
    limiter: Option<Semaphore>,
}

impl BucketController {
    pub fn new(
        bucket_id: u32,
        global_limiter: Option<Arc<Semaphore>>,
        config: BucketConfig,
        pipeline: PipelineController,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
            bucket_id,
            global_limiter,
            limiter: config.max_concurrency.map(Semaphore::new),
            config,
            pipeline,
            storage,
        }
    }
    
    #[inline]
    pub fn cfg(&self) -> &BucketConfig {
        &self.config
    }

    pub async fn upload(&self, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<UploadInfo> {
        let _permit = get_optional_permit(&self.global_limiter, &self.limiter).await?;
        let start = Instant::now();

        let checksum = crc32fast::hash(&data);
        let pipeline = self.pipeline.clone();
        let result = tokio::task::spawn_blocking(move || {
            pipeline.on_upload(kind, data)
        }).await??;

        println!(
            "{:?}",
            result
                .result
                .to_store
                .iter()
                .map(|v| (v.kind, v.sizing_id))
                .collect::<Vec<(ImageKind, u32)>>()
        );

        let image_id = Uuid::new_v4();
        for store_entry in result.result.to_store {
            self.storage
                .store(
                    self.bucket_id,
                    image_id,
                    store_entry.kind,
                    store_entry.sizing_id,
                    store_entry.data,
                ).await?;
        }

        Ok(UploadInfo {
            checksum,
            image_id,
            processing_time: start.elapsed().as_secs_f32(),
        })
    }

    pub async fn fetch(
        &self,
        image_id: Uuid,
        desired_kind: ImageKind,
        size_preset: Option<String>,
        custom_sizing: Option<(u32, u32)>,
    ) -> anyhow::Result<Option<StoreEntry>> {
        let _permit = get_optional_permit(&self.global_limiter, &self.limiter).await?;

        let sizing_id = size_preset.map(crate::utils::crc_hash).unwrap_or(0);

        // In real time situations
        let fetch_kind = if self.config.mode == ProcessingMode::Realtime {
            self.config.formats.original_image_store_format
        } else {
            desired_kind
        };

        let maybe_existing = self.storage.fetch(self.bucket_id, image_id, fetch_kind, sizing_id).await?;
        let (data, retrieved_kind) = match maybe_existing {
            // If we're in JIT mode we want to re-encode the image and store it.
            None => if self.config.mode == ProcessingMode::Jit {
                let base_kind = self.config.formats.original_image_store_format;
                let value = self.storage.fetch(
                    self.bucket_id,
                    image_id,
                    base_kind,
                    sizing_id,
                ).await?;

                match value {
                    None => return Ok(None),
                    Some(original) => (original, base_kind)
                }
            } else {
                return Ok(None)
            },
            Some(computed) => (computed, fetch_kind),
        };

        // Small optimisation here when in AOT mode to avoid
        // spawning additional threads.
        if self.config.mode == ProcessingMode::Aot {
            return Ok(Some(StoreEntry { data, kind: retrieved_kind, sizing_id }))
        }

        let pipeline = self.pipeline.clone();
        let result = tokio::task::spawn_blocking(move || {
            pipeline.on_fetch(desired_kind, retrieved_kind, data, sizing_id, custom_sizing)
        }).await??;

        for store_entry in result.result.to_store {
            self.storage.store(self.bucket_id, image_id, store_entry.kind, store_entry.sizing_id, store_entry.data).await?;
        }

        Ok(result.result.response)
    }

    pub async fn delete(&self, image_id: Uuid) -> anyhow::Result<()> {
        let _permit = get_optional_permit(&self.global_limiter, &self.limiter).await?;
        self.storage.delete(self.bucket_id, image_id).await
    }
}


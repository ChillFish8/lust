use std::sync::Arc;
use uuid::Uuid;
use poem_openapi::Object;
use tokio::sync::{Semaphore, SemaphorePermit};
use tokio::time::Instant;

use crate::config::{BucketConfig, ImageKind};
use crate::pipelines::{PipelineController, ProcessingMode};
use crate::storage::template::StorageBackend;


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
    global_limiter: Option<Arc<Semaphore>>,
    config: BucketConfig,
    pipeline: PipelineController,
    storage: Arc<dyn StorageBackend>,
    limiter: Option<Semaphore>,
}

impl BucketController {
    pub fn new(
        global_limiter: Option<Arc<Semaphore>>,
        config: BucketConfig,
        pipeline: PipelineController,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
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


        let image_id = Uuid::new_v4();
        for store_entry in result.result.to_store {
            self.storage.store(image_id, kind, store_entry).await?;
        }

        Ok(UploadInfo {
            checksum,
            image_id,
            processing_time: start.elapsed().as_secs_f32(),
        })
    }

    pub async fn fetch(&self, image_id: Uuid, kind: ImageKind) -> anyhow::Result<Option<Vec<u8>>> {
        let _permit = get_optional_permit(&self.global_limiter, &self.limiter).await?;

        let data = match self.storage.fetch(image_id, kind).await? {
            None => return Ok(None),
            Some(d) => d,
        };

        // Small optimisation here when in AOT mode to avoid
        // spawning additional threads.
        if self.config.mode == ProcessingMode::Aot {
            return Ok(Some(data))
        }

        let pipeline = self.pipeline.clone();
        let result = tokio::task::spawn_blocking(move || {
            pipeline.on_fetch(kind, data)
        }).await??;

        for store_entry in result.result.to_store {
            self.storage.store(image_id, kind, store_entry).await?;
        }

        Ok(result.result.response)
    }

    pub async fn delete(&self, image_id: Uuid) -> anyhow::Result<()> {
        let _permit = get_optional_permit(&self.global_limiter, &self.limiter).await?;
        self.storage.delete(image_id).await
    }
}


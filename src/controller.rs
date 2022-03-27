use std::sync::Arc;
use uuid::Uuid;
use poem_openapi::Object;
use tokio::sync::Semaphore;

use crate::config::{BucketConfig, ImageKind};
use crate::pipelines::PipelineController;
use crate::storage::template::StorageBackend;

macro_rules! get_optional_permit {
    ($limiter:expr) => {
        if let Some(limiter) = $limiter {
            Some(limiter.acquire().await?)
        } else {
            None
        }
    };
}

#[derive(Object, Debug)]
pub struct UploadInfo {
    /// The generated ID for the file.
    ///
    /// This can be used to access the file for the given bucket.
    file_id: Uuid,

    /// The time spent processing the image in seconds.
    processing_time: f32,

    /// The crc32 checksum of the uploaded image.
    checksum: u32,
}

pub struct BucketController {
    config: BucketConfig,
    pipeline: PipelineController,
    storage: Arc<dyn StorageBackend>,
    limiter: Option<Semaphore>,
}

impl BucketController {
    pub fn new(
        config: BucketConfig,
        pipeline: PipelineController,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
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
        let _permit = get_optional_permit!(&self.limiter);

        let pipeline = self.pipeline.clone();
        let result = tokio::task::spawn_blocking(move || {
            pipeline.on_upload(kind, data)
        }).await??;

        todo!()
    }

    pub async fn fetch(&self, image_id: Uuid, kind: ImageKind) -> anyhow::Result<Option<Vec<u8>>> {
        let _permit = get_optional_permit!(&self.limiter);

        let pipeline = self.pipeline.clone();
        let result = tokio::task::spawn_blocking(move || {
            pipeline.on_fetch(kind, todo!())
        }).await??;

        todo!()
    }

    pub async fn delete(&self, image_id: Uuid) -> anyhow::Result<()> {
        let _permit = get_optional_permit!(&self.limiter);

        todo!()
    }
}


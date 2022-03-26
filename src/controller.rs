use std::sync::Arc;
use uuid::Uuid;
use poem_openapi::Object;

use crate::config::{BucketConfig, ImageKind};
use crate::pipelines::PipelineController;
use crate::storage::template::StorageBackend;


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
}

impl BucketController {
    pub fn new(
        config: BucketConfig,
        pipeline: PipelineController,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
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
        todo!()
    }

    pub async fn fetch(&self, image_id: Uuid, kind: ImageKind) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    pub async fn delete(&self, image_id: Uuid) -> anyhow::Result<()> {
        todo!()
    }
}


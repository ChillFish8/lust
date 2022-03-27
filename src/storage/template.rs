use async_trait::async_trait;
use uuid::Uuid;
use crate::config::ImageKind;

#[async_trait]
pub trait StorageBackend: Sync + Send + 'static {
    async fn store(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
        data: Vec<u8>,
    ) -> anyhow::Result<()>;
    
    async fn fetch(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
    ) -> anyhow::Result<Option<Vec<u8>>>;
    
    async fn delete(
        &self,
        bucket_id: u32,
        image_id: Uuid,
    ) -> anyhow::Result<()>;
}
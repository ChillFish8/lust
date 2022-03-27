use async_trait::async_trait;
use uuid::Uuid;
use crate::config::ImageKind;

#[async_trait]
pub trait StorageBackend: Sync + Send + 'static {
    async fn store(&self, image_id: Uuid, kind: ImageKind, data: Vec<u8>) -> anyhow::Result<()>;
    
    async fn fetch(&self, image_id: Uuid, kind: ImageKind) -> anyhow::Result<Option<Vec<u8>>>;
    
    async fn delete(&self, image_id: Uuid) -> anyhow::Result<()>;
}
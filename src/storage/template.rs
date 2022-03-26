use async_trait::async_trait;

#[async_trait]
pub trait StorageBackend: Sync + Send + 'static {

}
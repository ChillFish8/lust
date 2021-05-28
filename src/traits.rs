use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ImageBackend {
    async fn add_image(file_id: Uuid, )
}
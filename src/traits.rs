use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use uuid::Uuid;

use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};

#[async_trait]
pub trait DatabaseLinker {
    async fn ensure_tables(&mut self, presets: Vec<&str>, columns: Vec<ImageFormat>) -> Result<()>;
}

#[async_trait]
pub trait ImageStore {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut>;

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()>;

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()>;

    async fn list_entities(
        &self,
        filter: FilterType,
        order: OrderBy,
        page: usize,
    ) -> Result<Vec<IndexResult>>;
}

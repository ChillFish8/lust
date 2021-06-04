use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;
use bytes::BytesMut;

use redis::{AsyncCommands, Client};
use redis::aio::ConnectionManager;

use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};



pub struct RedisConfig {
    connection_uri: String,
}


pub struct RedisBackend {
    client: Client,
    conn: ConnectionManager,
}

impl RedisBackend {
    pub async fn connect(cfg: RedisConfig) -> Result<Self> {
        let client = redis::Client::open(cfg.connection_uri)?;
        let conn = client.get_tokio_connection_manager().await?;

        Ok(Self {
            client,
            conn,
        })
    }
}

#[async_trait]
impl DatabaseLinker for RedisBackend {
    /// Due to the nature of the key-value setup for redis clients this has completely
    /// different handling so does not do anything when this funciton is called.
    async fn ensure_tables(&mut self, _presets: Vec<&str>, _columns: Vec<ImageFormat>) -> Result<()> {
        Ok(())
    }
}


#[async_trait]
impl ImageStore for RedisBackend {
    async fn get_image(&self, file_id: Uuid, preset: String, category: &str, format: ImageFormat) -> Option<BytesMut> {
        let hashable = (file_id, preset, category, format);
        hashable.hash
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {

    }

    /// This is non-functional due to limitations with the key-value setup of redis.
    async fn list_entities(&self, filter: FilterType, order: OrderBy, page: usize) -> Result<Vec<IndexResult>> {
        Err(anyhow::Error::msg("redis backend does not support listing entities"))
    }
}

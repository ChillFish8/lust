use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;
use bytes::BytesMut;
use serde::{Serialize, Deserialize};
use log::error;

use redis::{AsyncCommands, Client, AsyncIter};
use redis::aio::ConnectionManager;

use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};


#[derive(Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    connection_uri: String,
}


pub struct Backend {
    client: Client,
    conn: ConnectionManager,
}

impl Backend {
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
impl DatabaseLinker for Backend {
    /// Due to the nature of the key-value setup for redis clients this has completely
    /// different handling so does not do anything when this funciton is called.
    async fn ensure_tables(&mut self, _presets: Vec<&str>, _columns: Vec<ImageFormat>) -> Result<()> {
        Ok(())
    }
}


#[async_trait]
impl ImageStore for Backend {
    async fn get_image(&self, file_id: Uuid, preset: String, category: &str, format: ImageFormat) -> Option<BytesMut> {
        let key = format!("{:?} {} {} {:?}", file_id, preset, category, format);
        let mut conn = self.conn.clone();
        let result = conn.get(&key).await;

        let val: Vec<u8> = match result {
           Ok(v) => v,
           Err(e) => {
               error!("failed to fetch key {} from redis: {:?}", &key, e);
               return None
            }
        };

        if val.len() == 0 {
            None
        } else {
            let ref_: &[u8] = val.as_ref();
            Some(BytesMut::from(ref_))
        }
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        let mut pairs = Vec::new();

        for (preset, formats) in data {
            for (format, buff) in formats {
                let key = format!("{:?} {} {} {:?}", &file_id, &preset, category, format);
                pairs.push((key, buff.to_vec()));
            }
        }

        let mut conn = self.conn.clone();
        conn.set_multiple(&pairs).await?;

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, _presets: Vec<&String>) -> Result<()> {
        let mut conn = self.conn.clone();
        let mut keys: AsyncIter<String> = conn.scan_match(format!("{:?}*", file_id)).await?;
        while let Some(v) = keys.next_item().await {
            let mut conn_ = self.conn.clone();
            conn_.del(v).await?;
        }

        Ok(())
    }

    /// This is non-functional due to limitations with the key-value setup of redis.
    async fn list_entities(&self, _filter: FilterType, _order: OrderBy, _page: usize) -> Result<Vec<IndexResult>> {
        Err(anyhow::Error::msg("redis backend does not support listing entities"))
    }
}

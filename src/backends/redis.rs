use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use log::error;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, AsyncIter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};

#[derive(Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    connection_uri: String,
    pool_size: usize,
}

struct RedisPool {
    connections: Vec<ConnectionManager>,
    index: AtomicUsize,
}

impl RedisPool {
    pub async fn connect(cfg: RedisConfig) -> Result<Self> {
        let client = redis::Client::open(cfg.connection_uri)?;
        let mut conns = Vec::new();
        for _ in 0..cfg.pool_size {
            let conn = client.get_tokio_connection_manager().await?;
            conns.push(conn);
        }

        Ok(Self {
            connections: conns,
            index: AtomicUsize::new(0),
        })
    }

    pub fn get(&self) -> ConnectionManager {
        let index = self.index.load(Ordering::Relaxed);
        let conn = self.connections[index].clone();

        if index == (self.connections.len() - 1) {
            self.index.store(0, Ordering::Relaxed);
        } else {
            self.index.store(index + 1, Ordering::Relaxed);
        }

        conn
    }
}

pub struct Backend {
    pool: RedisPool,
}

impl Backend {
    pub async fn connect(cfg: RedisConfig) -> Result<Self> {
        let pool = RedisPool::connect(cfg).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseLinker for Backend {
    /// Due to the nature of the key-value setup for redis clients this has completely
    /// different handling so does not do anything when this funciton is called.
    async fn ensure_tables(
        &mut self,
        _presets: Vec<&str>,
        _columns: Vec<ImageFormat>,
    ) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl ImageStore for Backend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        let key = format!("{:?} {} {} {:?}", file_id, preset, category, format);
        let mut conn = self.pool.get();
        let result = conn.get(&key).await;

        let val: Vec<u8> = match result {
            Ok(v) => v,
            Err(e) => {
                error!("failed to fetch key {} from redis: {:?}", &key, e);
                return None;
            },
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

        let mut conn = self.pool.get();
        conn.set_multiple(&pairs).await?;

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, _presets: Vec<&String>) -> Result<()> {
        let mut conn = self.pool.get();
        let mut conn2 = self.pool.get();
        let mut keys: AsyncIter<String> = conn.scan_match(format!("{:?}*", file_id)).await?;
        while let Some(v) = keys.next_item().await {
            conn2.del(v).await?;
        }

        Ok(())
    }

    /// This is non-functional due to limitations with the key-value setup of redis.
    async fn list_entities(
        &self,
        _filter: FilterType,
        _order: OrderBy,
        _page: usize,
    ) -> Result<Vec<IndexResult>> {
        Err(anyhow::Error::msg(
            "redis backend does not support listing entities",
        ))
    }
}

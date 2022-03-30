use std::ops::Deref;
use anyhow::anyhow;
use bytes::Bytes;
use once_cell::sync::OnceCell;
use crate::config::CacheConfig;

static GLOBAL_CACHE: OnceCell<Cache> = OnceCell::new();

pub fn new_cache(cfg: CacheConfig) -> anyhow::Result<Option<Cache>> {
    if cfg.max_capacity.is_some() && cfg.max_images.is_some() {
        return Err(anyhow!("Cache must be *either* based off of number of images or amount of memory, not both."))
    } else if cfg.max_capacity.is_none() && cfg.max_images.is_none() {
        return Ok(None)
    }

    let mut cache = moka::sync::CacheBuilder::default();
    if let Some(max_items) = cfg.max_images {
        cache = cache.max_capacity(max_items as u64)
    }

    if let Some(max_memory) = cfg.max_capacity {
        cache = cache
            .weigher(|k: &String, v: &Bytes| (k.len() + v.len()) as u32)
            .max_capacity((max_memory * 1024 * 1024) as u64);
    }

    Ok(Some(cache.build().into()))
}

pub fn init_cache(cfg: CacheConfig) -> anyhow::Result<()> {
    if let Some(cache) = new_cache(cfg)? {
        let _ = GLOBAL_CACHE.set(cache);
    };
    Ok(())
}

pub fn global_cache<'a>() -> Option<&'a Cache> {
    GLOBAL_CACHE.get()
}

pub struct Cache {
    inner: moka::sync::Cache<String, Bytes>,
}

impl Deref for Cache {
    type Target = moka::sync::Cache<String, Bytes>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<moka::sync::Cache<String, Bytes>> for Cache {
    fn from(v: moka::sync::Cache<String, Bytes>) -> Self {
        Self {
            inner: v
        }
    }
}
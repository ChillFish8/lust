use bytes::BytesMut;
use concread::arcache::ARCache;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use uuid::Uuid;

use crate::image::ImageFormat;

/// The key that acts as the hashed key.
pub type CacheKey = (Uuid, String, ImageFormat);

/// Cheaply cloneable lock around a LRU cache.
pub type CacheStore = Arc<ARCache<CacheKey, BytesMut>>;

pub static CACHE_STATE: OnceCell<CacheState> = OnceCell::new();

/// A wrapper around the `CacheStore` type letting it be put into Gotham's
/// shared state.
#[derive(Clone)]
pub struct CacheState(pub CacheStore);

impl CacheState {
    /// Creates a new cache state instance with a given size.
    pub fn init(cache_size: usize) {
        let store = Arc::new(ARCache::new_size(cache_size, 12));
        let inst = Self { 0: store };

        let _ = CACHE_STATE.set(inst);
    }

    /// Get a item from the cache if it exists otherwise returns None.
    pub fn get(&self, file_id: Uuid, preset: String, format: ImageFormat) -> Option<BytesMut> {
        let ref_val = (file_id, preset, format);
        let target = self.0.read();
        target.get(&ref_val).map(|v| v.clone())
    }

    /// Adds an item to the cache, if the cache size is already at it's limit
    /// the least recently used (LRU) item is removed.
    pub fn set(&self, file_id: Uuid, preset: String, format: ImageFormat, data: BytesMut) {
        let ref_val = (file_id, preset, format);
        let mut target = self.0.write();
        target.insert(ref_val, data);
        target.commit();
    }
}

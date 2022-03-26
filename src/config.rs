use std::collections::HashMap;
use std::path::Path;
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;

use crate::storage::backends::BackendConfigs;

static CONFIG: OnceCell<RuntimeConfig> = OnceCell::new();

pub fn config() -> &'static RuntimeConfig {
    CONFIG.get().expect("config init")
}

pub async fn init(config_file: &Path) -> Result<()> {
    let file = tokio::fs::read(config_file).await?;

    if let Some(ext) = config_file.extension() {
        let ext = ext.to_string_lossy().to_string();
        let cfg: RuntimeConfig = match ext.as_str() {
            "json" => serde_json::from_slice(&file)?,
            "yaml" => serde_yaml::from_slice(&file)?,
            _ => return Err(anyhow!("Config file must have an extension of either `.json` or `.yaml`"))
        };

        let _ = CONFIG.set(cfg);
        Ok(())
    } else {
        Err(anyhow!("Config file must have an extension of either `.json` or `.yaml`"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingMode {
    /// Images will be optimised and resized when they're
    /// requested and then stored.
    Jit,

    /// Images have all optimizations and resizing applied to them
    /// and stored at upload time.
    Aot,

    /// Only the original image will be stored, any optimisations will always
    /// be ran at request time and not stored.
    Realtime,
}

impl Default for ProcessingMode {
    fn default() -> Self {
        Self::Jit
    }
}

#[derive(Debug, Deserialize)]
pub struct RuntimeConfig {
    /// The set storage backend configuration.
    pub backend: BackendConfigs,

    /// A set of bucket configs.
    ///
    /// Each bucket represents a category.
    pub buckets: HashMap<String, BucketConfig>,

    /// The base path to serve images from.
    ///
    /// Defaults to `/images`.
    pub base_serving_path: String,

    /// The global cache handler.
    ///
    /// This will be the fallback handler if any buckets are not
    /// assigned a dedicated cache config.
    ///
    /// If this is `None` then no caching is performed.
    pub global_cache: Option<CacheConfig>,

    /// The *global* max upload size allowed for this bucket in MB.
    ///
    /// This takes precedence over bucket level limits.
    pub max_upload_size: Option<usize>,
}

impl RuntimeConfig {
    #[inline]
    pub fn valid_global_size(&self, size: usize) -> bool {
        self
            .max_upload_size
            .map(|limit| size <= limit)
            .unwrap_or(false)
    }
}

#[derive(Debug, Deserialize)]
pub struct CacheConfig {
    /// The maximum amount of images to cache.
    ///
    /// If set to `None` then this will fall back to capacity
    /// based caching.
    ///
    /// If both entries are `None` then the item is not cached.
    pub max_images: Option<u16>,

    /// The maximum amount of memory (approximately) in MB.
    ///
    /// If set to `None` then this will fall back to
    /// number of entries based caching.
    ///
    /// If both entries are `None` then the item is not cached.
    pub max_capacity: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct BucketConfig {
    #[serde(default)]
    /// The processing mode for the given bucket.
    ///
    /// See `config::ProcessingMode` for more.
    pub mode: ProcessingMode,

    /// The given image format optimisation config.
    pub formats: ImageFormats,

    /// The default format to serve images as.
    ///
    /// Defaults to the first enabled encoding format.
    pub default_serving_format: Option<ImageKind>,

    #[serde(default = "default_preset")]
    /// The default resizing preset to serve images as.
    ///
    /// Defaults to "original".
    pub default_serving_preset: String,

    #[serde(default)]
    /// A set of resizing presets, this allows resizing dimensions to be accessed
    /// via a name. E.g. "small", "medium", "large", etc...
    pub presets: HashMap<String, ResizingConfig>,

    /// A local cache config.
    ///
    /// If `None` this will use the global handler.
    pub cache: Option<CacheConfig>,

    /// The max upload size allowed for this bucket in MB.
    pub max_upload_size: Option<u32>,
}


#[derive(Debug, Deserialize, strum::AsRefStr)]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    Png,
    Jpeg,
    Webp,
    Gif,
}


#[derive(Debug, Deserialize)]
pub struct ImageFormats {
    #[serde(default = "default_true")]
    /// Enable PNG re-encoding.
    ///
    /// Defaults to `true`.
    pub png: bool,

    #[serde(default = "default_true")]
    /// Enable JPEG re-encoding.
    ///
    /// Defaults to `true`.
    pub jpeg: bool,

    #[serde(default = "default_true")]
    /// Enable WebP re-encoding.
    ///
    /// Defaults to `true`.
    pub webp: bool,

    #[serde(default)]
    /// Enable gif re-encoding.
    ///
    /// This is generally quite a slow encoder and generally
    /// not recommended for most buckets.
    ///
    /// Defaults to `false`.
    pub gif: bool,

    #[serde(default)]
    /// The (optional) webp encoder config.
    ///
    /// This is used for fine-tuning the webp encoder for a desired size and
    /// performance behavour.
    pub webp_config: WebpConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct WebpConfig {
    /// The ratio of lossy compression for webp images
    /// from 0.0 to 100.0 inclusive for minimal and maximal quality respectively.
    ///
    /// This can be set to null to put the encoder into lossless compression mode.
    pub quality: Option<f32>,

    /// with lossless encoding is the ratio of compression to speed.
    /// If using lossy encoding this does nothing - (float: 0.0 - 100.0 inclusive).
    pub compression: Option<f32>,

    /// The quality/speed trade-off (0=fast, 6=slower-better)
    pub method: Option<u8>,

    #[serde(default)]
    /// A bool singling if multi-threading encoding should be attempted.
    pub threading: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct ResizingConfig {
    /// The width to resize the image to.
    pub width: u16,

    /// The height to resize the image to.
    pub height: u16,
}

const fn default_true() -> bool {
    true
}

fn default_preset() -> String {
    String::from("original")
}

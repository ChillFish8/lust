use std::collections::HashMap;
use serde::Deserialize;

use crate::storage::backends::BackendConfigs;

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
    pub buckets: Vec<BucketConfig>,
}


#[derive(Debug, Deserialize)]
pub struct BucketConfig {
    #[serde(default)]
    /// The processing mode for the given bucket.
    ///
    /// See `config::ProcessingMode` for more.
    pub mode: ProcessingMode,

    /// The given image format optimisation config.
    pub formats: Formats,

    #[serde(default)]
    /// A set of resizing presets, this allows resizing dimensions to be accessed
    /// via a name. E.g. "small", "medium", "large", etc...
    pub presets: HashMap<String, ResizingConfig>,
}


#[derive(Debug, Deserialize)]
pub struct Formats {
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

    /// A bool singling if multi-threading encoding should be attempted.
    pub threading: Option<bool>,
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
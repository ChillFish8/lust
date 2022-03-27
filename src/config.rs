use std::collections::HashMap;
use std::path::Path;
use anyhow::{anyhow, Result};
use image::ImageFormat;
use image::imageops::FilterType;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use poem_openapi::Enum;
use crate::pipelines::ProcessingMode;

use crate::storage::backends::BackendConfigs;

static CONFIG: OnceCell<RuntimeConfig> = OnceCell::new();

pub fn config() -> &'static RuntimeConfig {
    CONFIG.get().expect("config init")
}

#[cfg(test)]
pub fn init_test(data: &str) -> Result<()> {
    let cfg: RuntimeConfig = serde_yaml::from_str(data)?;
    dbg!(&cfg);
    let _ = CONFIG.set(cfg);
    Ok(())
}

pub async fn init(config_file: &Path) -> Result<()> {
    let file = tokio::fs::read(config_file).await?;

    if let Some(ext) = config_file.extension() {
        let ext = ext.to_string_lossy().to_string();
        let cfg: RuntimeConfig = match ext.as_str() {
            "json" => serde_json::from_slice(&file)?,
            "yaml" => serde_yaml::from_slice(&file)?,
            "yml" => serde_yaml::from_slice(&file)?,
            _ => return Err(anyhow!("Config file must have an extension of either `.json`,`.yaml` or `.yml`"))
        };

        let _ = CONFIG.set(cfg);
        Ok(())
    } else {
        Err(anyhow!("Config file must have an extension of either `.json` or `.yaml`"))
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

    /// The global max concurrency.
    ///
    /// This takes precedence over bucket level limits.
    pub max_concurrency: Option<usize>,
}

impl RuntimeConfig {
    #[inline]
    pub fn valid_global_size(&self, size: usize) -> bool {
        self
            .max_upload_size
            .map(|limit| size <= limit)
            .unwrap_or(true)
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

    /// The per-bucket max concurrency.
    pub max_concurrency: Option<usize>,
}

impl BucketConfig {
    #[inline]
    pub fn sizing_preset_ids(&self) -> Vec<u32> {
        self.presets.keys()
            .map(crate::utils::crc_hash)
            .collect()
    }
}

#[derive(Copy, Clone, Debug, Enum, Eq, PartialEq, Deserialize, strum::AsRefStr)]
#[oai(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ImageKind {
    /// The PNG encoding format.
    Png,

    /// The JPEG encoding format.
    Jpeg,

    /// The WebP encoding format.
    Webp,

    /// The GIF encoding format.
    Gif,
}

#[allow(clippy::from_over_into)]
impl Into<image::ImageFormat> for ImageKind {
    fn into(self) -> ImageFormat {
        match self {
            Self::Png => image::ImageFormat::Png,
            Self::Jpeg => image::ImageFormat::Jpeg,
            Self::Gif => image::ImageFormat::Gif,
            Self::Webp => image::ImageFormat::WebP,
        }
    }
}

impl ImageKind {
    pub fn from_content_type(kind: &str) -> Option<Self> {
        match kind {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/gif" => Some(Self::Gif),
            "image/webp" => Some(Self::Webp),
            "png" => Some(Self::Png),
            "jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::Webp),
            _ => None
        }
    }

    pub fn from_guessed_format(fmt: image::ImageFormat) -> Option<Self> {
        match fmt {
            image::ImageFormat::Png => Some(Self::Png),
            image::ImageFormat::Jpeg => Some(Self::Jpeg),
            image::ImageFormat::Gif => Some(Self::Gif),
            image::ImageFormat::WebP => Some(Self::Webp),
            _ => None
        }
    }

    pub fn as_content_type(&self) -> String {
        format!("image/{}", self.as_file_extension())
    }

    pub fn as_file_extension(&self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpeg",
            ImageKind::Webp => "webp",
            ImageKind::Gif => "gif",
        }
    }

    pub fn variants() -> &'static [Self] {
        &[
            Self::Png,
            Self::Jpeg,
            Self::Gif,
            Self::Webp,
        ]
    }
}


#[derive(Copy, Clone, Debug, Deserialize)]
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

    #[serde(default = "default_original_format")]
    /// The format to encode and store the original image as.
    ///
    /// This is only used for the JIT and Realtime processing modes
    /// and will default to PNG encoding if empty.
    pub original_image_store_format: ImageKind,
}

impl ImageFormats {
    pub fn is_enabled(&self, kind: ImageKind) -> bool {
        match kind {
            ImageKind::Png => self.png,
            ImageKind::Jpeg => self.jpeg,
            ImageKind::Webp => self.webp,
            ImageKind::Gif => self.gif,
        }
    }

    pub fn first_enabled_format(&self) -> ImageKind {
        if self.png {
            return ImageKind::Png
        }

        if self.jpeg {
            return ImageKind::Jpeg
        }

        if self.webp {
            return ImageKind::Webp
        }

        if self.gif {
            return ImageKind::Gif
        }

        panic!("Invalid configuration, expected at least one enabled format.")
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
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

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum ResizingFilter {
    /// Nearest Neighbor
    Nearest,

    /// Linear Filter
    Triangle,

    /// Cubic Filter
    CatmullRom,

    /// Gaussian Filter
    Gaussian,

    /// Lanczos with window 3
    Lanczos3,
}

#[allow(clippy::from_over_into)]
impl Into<image::imageops::FilterType> for ResizingFilter {
    fn into(self) -> FilterType {
        match self {
            ResizingFilter::Nearest => FilterType::Nearest,
            ResizingFilter::Triangle => FilterType::Triangle,
            ResizingFilter::CatmullRom => FilterType::CatmullRom,
            ResizingFilter::Gaussian => FilterType::Gaussian,
            ResizingFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

impl Default for ResizingFilter {
    fn default() -> Self {
        Self::Nearest
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub struct ResizingConfig {
    /// The width to resize the image to.
    pub width: u32,

    /// The height to resize the image to.
    pub height: u32,

    #[serde(default)]
    /// The resizing filter algorithm to use.
    ///
    /// Defaults to nearest neighbour.
    pub filter: ResizingFilter,
}

const fn default_true() -> bool {
    true
}

const fn default_original_format() -> ImageKind {
    ImageKind::Png
}

fn default_preset() -> String {
    String::from("original")
}

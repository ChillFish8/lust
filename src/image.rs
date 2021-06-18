use anyhow::Result;
use bytes::{BufMut, BytesMut};
use gotham::state::{FromState, State};
use gotham_derive::{StateData, StaticResponseExtender};
use hashbrown::HashMap;
use log::error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webp::Encoder;

use image::imageops;
use image::{load_from_memory_with_format, DynamicImage};

use crate::configure::StateConfig;
use crate::storage::StorageBackend;
use crate::traits::ImageStore;

pub type ImageData = HashMap<ImageFormat, BytesMut>;
pub type ImagePresetsData = HashMap<String, ImageData>;

pub type ImageDataSizes = HashMap<ImageFormat, usize>;
pub type ImagePresetDataSizes = HashMap<String, ImageDataSizes>;

#[derive(Debug, Clone, Ord, PartialOrd, Hash, Eq, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    WebP,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct ImageGet {
    pub format: Option<ImageFormat>,
    pub encode: Option<bool>,
    pub preset: Option<String>,
}

#[derive(Deserialize)]
pub struct ImageUpload {
    pub format: ImageFormat,
    pub data: String,
    pub category: Option<String>,
}

#[derive(Serialize)]
pub struct ImageUploaded {
    pub file_id: Uuid,
    pub formats: ImagePresetDataSizes,
    pub category: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct ImageRemove {
    pub file_id: Uuid,
}

macro_rules! convert {
    ( $e:expr, $d:expr ) => {{
        || -> anyhow::Result<BytesMut> {
            let buff = BytesMut::new();
            let mut writer = buff.writer();
            $e.write_to(&mut writer, $d)?;
            Ok(writer.into_inner())
        }()
    }};
}

macro_rules! generate {
    ( $n:expr, $e:expr, $hm1:expr, $hm2:expr, $cfg:expr ) => ({
        let (data, sizes) = convert_image(&$e, $cfg)?;
        $hm1.insert($n.to_string(), sizes);
        $hm2.insert($n.to_string(), data);
    })
}

macro_rules! is_enabled {
    ( $format:expr, $options:expr ) => {{
        $options.get(&$format).map(|v| *v).unwrap_or(true)
    }};
}

macro_rules! log_err {
    ( $result:expr, $msg:expr ) => {{
        match &$result {
            Ok(_) => (),
            Err(e) => error!("{} {:?}", $msg, e),
        };

        $result
    }};
}

fn convert_image(im: &DynamicImage, cfg: StateConfig) -> Result<(ImageData, ImageDataSizes)> {
    let mut resulting_sizes = HashMap::with_capacity(4);
    let mut resulting_data = HashMap::with_capacity(4);

    if is_enabled!(ImageFormat::Png, cfg.0.formats) {
        let png: BytesMut = log_err!(
            convert!(&im, image::ImageFormat::Png),
            "failed to convert png: "
        )?;
        resulting_sizes.insert(ImageFormat::Png, png.len());
        resulting_data.insert(ImageFormat::Png, png);
    }

    if is_enabled!(ImageFormat::Jpeg, cfg.0.formats) {
        let jpeg = log_err!(
            convert!(&im, image::ImageFormat::Jpeg),
            "failed to convert jpeg: "
        )?;
        resulting_sizes.insert(ImageFormat::Jpeg, jpeg.len());
        resulting_data.insert(ImageFormat::Jpeg, jpeg);
    }

    if is_enabled!(ImageFormat::Gif, cfg.0.formats) {
        let gif = log_err!(
            convert!(&im, image::ImageFormat::Gif),
            "failed to convert gif: "
        )?;
        resulting_sizes.insert(ImageFormat::Gif, gif.len());
        resulting_data.insert(ImageFormat::Gif, gif);
    }

    // This is the slowest conversion, maybe change??
    let start = std::time::Instant::now();
    if is_enabled!(ImageFormat::WebP, cfg.0.formats) {
        let raw = if let Some(quality) = cfg.0.webp_quality {
            Encoder::from_image(&im).encode(quality)
        } else {
            Encoder::from_image(&im).encode_lossless()
        };
        let webp = BytesMut::from(raw.as_ref());
        resulting_sizes.insert(ImageFormat::WebP, webp.len());
        resulting_data.insert(ImageFormat::WebP, webp);
    }
    println!("{:?}", start.elapsed());

    Ok((resulting_data, resulting_sizes))
}

pub async fn process_new_image(
    state: &mut State,
    category: &str,
    format: ImageFormat,
    data: Vec<u8>,
) -> Result<(Uuid, ImagePresetDataSizes)> {
    let cfg = StateConfig::take_from(state);
    let storage = StorageBackend::take_from(state);

    let fmt = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Gif => image::ImageFormat::Gif,
        ImageFormat::WebP => image::ImageFormat::WebP,
    };

    let presets = &cfg.0.size_presets;
    let mut converted_sizes = HashMap::with_capacity(presets.len());
    let mut converted_data = HashMap::with_capacity(presets.len());
    let original = log_err!(
        load_from_memory_with_format(&data, fmt),
        "failed to load format due to exception: "
    )?;
    generate!(
        "original",
        original,
        converted_sizes,
        converted_data,
        cfg.clone()
    );

    for (preset_name, size) in presets {
        let im = original.resize(size.width, size.height, imageops::FilterType::Nearest);

        generate!(
            preset_name,
            im,
            converted_sizes,
            converted_data,
            cfg.clone()
        );
    }

    let file_id = Uuid::new_v4();
    storage.add_image(file_id, category, converted_data).await?;

    Ok((file_id, converted_sizes))
}

pub async fn get_image(
    state: &mut State,
    file_id: Uuid,
    preset: String,
    category: &str,
    format: ImageFormat,
) -> Option<BytesMut> {
    let storage = StorageBackend::take_from(state);
    storage.get_image(file_id, preset, category, format).await
}

pub async fn delete_image(state: &mut State, file_id: Uuid) -> Result<()> {
    let storage = StorageBackend::take_from(state);
    let cfg = StateConfig::take_from(state);

    let presets = cfg.0.size_presets.keys().collect();
    storage.remove_image(file_id, presets).await?;

    Ok(())
}

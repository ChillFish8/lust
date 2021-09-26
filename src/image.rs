use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use bytes::{BufMut, BytesMut};
use gotham::state::{FromState, State};
use gotham_derive::{StateData, StaticResponseExtender};
use hashbrown::HashMap;
use image::{imageops, load_from_memory_with_format, DynamicImage};
use log::{debug, error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webp::Encoder;

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
            let start = Instant::now();
            $e.write_to(&mut writer, $d)?;
            debug!("format {:?} conversion took {:?}", $d, start.elapsed());
            Ok(writer.into_inner())
        }()
    }};
}

macro_rules! generate {
    ( $n:expr, $e:expr, $hm1:expr, $hm2:expr, $cfg:expr ) => ({
        let (data, sizes) = convert_image($e, $cfg).await?;
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

fn spawn_conversion(
    img: Arc<DynamicImage>,
    format: ImageFormat,
    convert_to_format: image::ImageFormat,
) -> Result<(ImageFormat, BytesMut)> {
    let img: BytesMut = log_err!(
        convert!(img, convert_to_format),
        format!("failed to convert {:?}: ", convert_to_format)
    )?;

    return Ok((format, img));
}

async fn convert_image(
    img: Arc<DynamicImage>,
    cfg: StateConfig,
) -> Result<(ImageData, ImageDataSizes)> {
    let mut resulting_sizes = HashMap::with_capacity(4);
    let mut resulting_data = HashMap::with_capacity(4);

    let mut handles = vec![];

    if is_enabled!(ImageFormat::Png, cfg.0.formats) {
        let cloned = img.clone();
        let handle = tokio::task::spawn_blocking(move || {
            spawn_conversion(cloned, ImageFormat::Png, image::ImageFormat::Png)
        });
        handles.push(handle);
    }

    if is_enabled!(ImageFormat::Jpeg, cfg.0.formats) {
        let cloned = img.clone();
        let handle = tokio::task::spawn_blocking(move || {
            spawn_conversion(cloned, ImageFormat::Jpeg, image::ImageFormat::Jpeg)
        });
        handles.push(handle);
    }

    if is_enabled!(ImageFormat::Gif, cfg.0.formats) {
        let cloned = img.clone();
        let handle = tokio::task::spawn_blocking(move || {
            spawn_conversion(cloned, ImageFormat::Gif, image::ImageFormat::Gif)
        });
        handles.push(handle);
    }

    // This is the slowest conversion, maybe change??
    // Updated: New encoder allows for multi threading encoding.
    if is_enabled!(ImageFormat::WebP, cfg.0.formats) {
        let cloned = img.clone();
        let handle = tokio::task::spawn_blocking(move || -> Result<(ImageFormat, BytesMut)> {
            let start = Instant::now();
            let raw = Encoder::from_image(cloned.as_ref()).encode();
            debug!(
                "format {:?} conversion took {:?}",
                image::ImageFormat::WebP,
                start.elapsed()
            );
            let webp = BytesMut::from(raw.as_ref());

            Ok((ImageFormat::WebP, webp))
        });
        handles.push(handle);
    }

    for handle in handles {
        let (format, data) = handle.await??;
        resulting_sizes.insert(format, data.len());
        resulting_data.insert(format, data);
    }

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
    let original = Arc::from(log_err!(
        load_from_memory_with_format(&data, fmt),
        "failed to load format due to exception: "
    )?);
    generate!(
        "original",
        original.clone(),
        converted_sizes,
        converted_data,
        cfg.clone()
    );

    for (preset_name, size) in presets {
        let cloned = original.clone();
        let im = Arc::new(cloned.resize(size.width, size.height, imageops::FilterType::Nearest));

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

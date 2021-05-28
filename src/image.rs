use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use gotham::state::{FromState, State};
use hashbrown::HashMap;
use uuid::Uuid;
use webp::Encoder;

use image::imageops;
use image::{load_from_memory_with_format, DynamicImage};

use flate2::bufread::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

use crate::configure::StateConfig;
use crate::context::{ImageData, ImageDataSizes, ImageFormat, ImagePresetDataSizes};
use crate::storage::StorageBackend;
use crate::traits::ImageStore;

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

macro_rules! compress {
    ( $e:expr ) => {{
        GzEncoder::new($e.reader(), Compression::fast())
            .into_inner()
            .into_inner()
    }};
}

macro_rules! generate {
    ( $n:expr, $e:expr, $hm1:expr, $hm2:expr ) => ({
        let (data, sizes) = convert_image(&$e)?;
        $hm1.insert($n.to_string(), sizes);
        $hm2.insert($n.to_string(), data);
    })
}

fn convert_image(im: &DynamicImage) -> Result<(ImageData, ImageDataSizes)> {
    let png = convert!(&im, image::ImageFormat::Png)?;
    let jpeg = convert!(&im, image::ImageFormat::Jpeg)?;
    let gif = convert!(&im, image::ImageFormat::Gif)?;
    let webp = BytesMut::from(Encoder::from_image(&im).encode_lossless().as_ref());

    let compressed_png = compress!(png);
    let compressed_jpeg = compress!(jpeg);
    let compressed_gif = compress!(gif);
    let compressed_webp = compress!(webp);

    let mut resulting_sizes = HashMap::with_capacity(4);
    resulting_sizes.insert(ImageFormat::Png, compressed_png.len());
    resulting_sizes.insert(ImageFormat::Jpeg, compressed_jpeg.len());
    resulting_sizes.insert(ImageFormat::Gif, compressed_gif.len());
    resulting_sizes.insert(ImageFormat::WebP, compressed_webp.len());

    let mut resulting_data = HashMap::with_capacity(4);
    resulting_data.insert(ImageFormat::Png, compressed_png);
    resulting_data.insert(ImageFormat::Jpeg, compressed_jpeg);
    resulting_data.insert(ImageFormat::Gif, compressed_gif);
    resulting_data.insert(ImageFormat::WebP, compressed_webp);

    Ok((resulting_data, resulting_sizes))
}

pub async fn process_new_image(
    state: &mut State,
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
    let original = load_from_memory_with_format(&data, fmt)?;
    generate!("original", original, converted_sizes, converted_data);

    for (preset_name, size) in presets {
        let im = original.resize(size.width, size.height, imageops::FilterType::Nearest);

        generate!(preset_name, im, converted_sizes, converted_data);
    }

    let file_id = Uuid::new_v4();
    storage.add_image(file_id, converted_data).await?;

    Ok((file_id, converted_sizes))
}

pub async fn get_image(
    state: &mut State,
    file_id: Uuid,
    preset: String,
    format: ImageFormat,
    compress: bool,
) -> Option<BytesMut> {
    let storage = StorageBackend::take_from(state);

    if let Some(mut buff) = storage.get_image(file_id, preset, format).await {
        if !compress {
            buff = GzDecoder::new(buff.reader()).into_inner().into_inner();
        }

        Some(buff)
    } else {
        None
    }
}

pub async fn delete_image(state: &mut State, file_id: Uuid) -> Result<()> {
    let storage = StorageBackend::take_from(state);

    storage.remove_image(file_id).await?;

    Ok(())
}

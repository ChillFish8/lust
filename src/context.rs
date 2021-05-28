use bytes::BytesMut;
use hashbrown::HashMap;
use uuid::Uuid;

use gotham_derive::{StateData, StaticResponseExtender};
use serde::{Deserialize, Serialize};

pub type ImageData = HashMap<ImageFormat, BytesMut>;
pub type ImagePresetsData = HashMap<String, ImageData>;

pub type ImageDataSizes = HashMap<ImageFormat, usize>;
pub type ImagePresetDataSizes = HashMap<String, ImageDataSizes>;

#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    WebP,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionMode {
    Always,
    Never,
    Auto,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct ImageGet {
    pub format: Option<ImageFormat>,
    pub encode: Option<bool>,
    pub compress: Option<bool>,
    pub preset: Option<String>,
}

#[derive(Deserialize)]
pub struct ImageUpload {
    pub format: ImageFormat,
    pub data: String,
    pub compressed: Option<bool>,
}

#[derive(Serialize)]
pub struct ImageUploaded {
    pub file_id: Uuid,
    pub formats: ImagePresetDataSizes,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct ImageRemove {
    pub file_id: Uuid,
}

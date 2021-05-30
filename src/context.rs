use bytes::BytesMut;
use hashbrown::HashMap;
use uuid::Uuid;
use std::time::SystemTime;

use gotham_derive::{StateData, StaticResponseExtender};
use serde::{Deserialize, Serialize};

pub type ImageData = HashMap<ImageFormat, BytesMut>;
pub type ImagePresetsData = HashMap<String, ImageData>;

pub type ImageDataSizes = HashMap<ImageFormat, usize>;
pub type ImagePresetDataSizes = HashMap<String, ImageDataSizes>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Copy)]
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

/// A set of filters that can be used to view
/// entities via the REST API on the admin panel.
///
/// Example:
///
/// ```json
/// {
///     "filter": {
///         "filter_type": "category",
///         "with_value": "cats",
///     }
/// }
/// ```
#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "filter_type", content = "with_value")]
pub enum FilterType {
    All,
    Category(String),
    CreationDate(SystemTime)
}


/// How the data should be ordered when requesting the
/// index list.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderBy {
    CreationDate,
    TotalSize,
}

/// A result when listing all items in the server.
#[derive(Serialize)]
pub struct IndexResult {
    file_id: Uuid,
    total_size: usize,
    created_on: SystemTime,
}
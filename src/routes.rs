use std::fmt::Display;
use bytes::Bytes;
use poem_openapi::OpenApi;
use poem::{Body, Result};
use poem_openapi::{ApiResponse, Object};
use poem_openapi::param::{Header, Path, Query};
use poem_openapi::payload::{Binary, Json};
use futures::StreamExt;
use uuid::Uuid;

use crate::config::{config, ImageKind};
use crate::controller::{BucketController, get_bucket_by_name, UploadInfo};
use crate::pipelines::ProcessingMode;


#[derive(Debug, Object)]
pub struct Detail {
    /// Additional information regarding the response.
    detail: String,
}


#[derive(ApiResponse)]
pub enum UploadResponse {
    #[oai(status = 200)]
    Ok(Json<UploadInfo>),

    /// Bucket not found
    #[oai(status = 404)]
    NotFound,

    /// The upload is missing the content-type header.
    #[oai(status = 411)]
    MissingHeader,

    /// The upload exceeds the configured maximum file size.
    #[oai(status = 413)]
    TooBig,

    /// The given content-type is not allowed.
    ///
    /// The type must be one of:
    /// - `image/gif`
    /// - `image/jpeg`
    /// - `image/png`
    /// - `image/webp`
    #[oai(status = 415)]
    InvalidContentType,

    #[allow(unused)]
    /// You are not authorized to complete this action.
    ///
    /// This normally means the `Authorization` bearer has been left out
    /// of the request or is invalid.
    #[oai(status = 401)]
    Unauthorized,
}

#[derive(ApiResponse)]
pub enum DeleteResponse {
    #[oai(status = 200)]
    Ok,


    #[allow(unused)]
    /// You are not authorized to complete this action.
    ///
    /// This normally means the `Authorization` bearer has been left out
    /// of the request or is invalid.
    #[oai(status = 401)]
    Unauthorized,

    /// Bucket does not exist.
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
pub enum FetchResponse {
    #[oai(status = 200)]
    Ok(
        Binary<Vec<u8>>,
        #[oai(header = "content-type")] String,
    ),

    /// The request is invalid with the current configuration.
    ///
    /// See the detail section for more info.
    #[oai(status = 400)]
    UnsupportedOperation(Json<Detail>),

    /// Bucket does not exist or image does not exist.
    ///
    /// See the detail section for more info.
    #[oai(status = 404)]
    NotFound(Json<Detail>),
}

impl FetchResponse {
    fn bucket_not_found(bucket: &str) -> Self {
        let detail = Detail {
            detail: format!("The bucket {:?} does not exist.", bucket),
        };

        Self::NotFound(Json(detail))
    }

    fn image_not_found(image_id: Uuid) -> Self {
        let detail = Detail {
            detail: format!("The image {:?} does not exist in bucket.", image_id),
        };

        Self::NotFound(Json(detail))
    }

    fn bad_request(msg: impl Display) -> Self {
        let detail = Detail {
            detail: msg.to_string(),
        };

        Self::UnsupportedOperation(Json(detail))
    }
}


pub struct LustApi ;

#[OpenApi(prefix_path = "/:bucket")]
impl LustApi {
    /// Upload Image
    ///
    /// Upload an image to the given bucket.
    /// The `content-type` header must be provided as well
    /// as the `content-length` header otherwise the request will be rejected.
    ///
    /// The uploaded file must also not exceed the given `content-length`.
    #[oai(path = "/", method = "post")]
    pub async fn upload_image(
        &self,
        bucket: Path<String>,
        #[oai(name = "content-length")] content_length: Header<usize>,
        #[oai(name = "content-type")] content_type: Header<String>,
        file: Binary<Body>,
    ) -> Result<UploadResponse> {
        let bucket = match get_bucket_by_name(&*bucket) {
            None => return Ok(UploadResponse::NotFound),
            Some(b) => b,
        };

        let format: ImageKind = match ImageKind::from_content_type(&*content_type) {
            None => return Ok(UploadResponse::InvalidContentType),
            Some(f) => f,
        };

        let length = if !config().valid_global_size(*content_length) {
            return Ok(UploadResponse::TooBig)
        } else {
            let local_limit = bucket
                .cfg()
                .max_upload_size
                .map(|v| v as usize)
                .unwrap_or(u32::MAX as usize);

            if *content_length > local_limit  {
                return Ok(UploadResponse::TooBig)
            }

            *content_length
        };

        let mut allocated_image = Vec::with_capacity(length);
        let mut stream = file.0.into_bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk: Bytes = chunk.map_err(anyhow::Error::from)?;
            allocated_image.extend(chunk.into_iter());

            if allocated_image.len() > length {
                return Ok(UploadResponse::TooBig)
            }
        }

        let info = bucket.upload(format, allocated_image).await?;
        Ok(UploadResponse::Ok(Json(info)))
    }

    /// Fetch Image
    ///
    /// Fetch the image from the storage backend and apply and additional affects
    /// if required.
    #[allow(clippy::too_many_arguments)]
    #[oai(path = "/:image_id", method = "get")]
    pub async fn fetch_image(
        &self,
        bucket: Path<String>,
        image_id: Path<Uuid>,
        format: Query<Option<ImageKind>>,
        size: Query<Option<String>>,
        width: Query<Option<u32>>,
        height: Query<Option<u32>>,
        accept: Header<Option<String>>,
    ) -> Result<FetchResponse> {
        let bucket = match get_bucket_by_name(&*bucket) {
            None => return Ok(FetchResponse::bucket_not_found(&*bucket)),
            Some(b) => b,
        };

        let kind = get_image_kind(format.0, accept.0, bucket);
        let custom_sizing = match (width.0, height.0) {
            (Some(w), Some(h)) => if bucket.cfg().mode != ProcessingMode::Realtime {
                return Ok(FetchResponse::bad_request(
                    "Custom resizing can only be done when bucket set to 'realtime' processing mode",
                ))
            } else {
                Some((w, h))
            },
            (None, None) => None,
            _ => return Ok(FetchResponse::bad_request(
                "A custom size must include both the width and the height.",
            ))
        };

        let img = bucket.fetch(image_id.0, kind, size.0, custom_sizing).await?;
        match img {
            None => Ok(FetchResponse::image_not_found(image_id.0)),
            Some(img) => Ok(FetchResponse::Ok(Binary(img.data), img.kind.as_content_type()))
        }
    }

    /// Delete Image
    ///
    /// Delete the given image.
    /// This will purge all variants of the image including sizing presets and formats.
    ///
    /// Images that do not exist already will be ignored and will not return a 404.
    #[oai(path = "/:image_id", method = "delete")]
    pub async fn delete_image(
        &self,
        bucket: Path<String>,
        image_id: Path<Uuid>,
    ) -> Result<DeleteResponse> {
        let bucket = match get_bucket_by_name(&*bucket) {
            None => return Ok(DeleteResponse::NotFound),
            Some(b) => b,
        };

        bucket.delete(*image_id).await?;

        Ok(DeleteResponse::Ok)
    }
}


fn get_image_kind(direct_format: Option<ImageKind>, accept: Option<String>, bucket: &BucketController) -> ImageKind {
    match direct_format {
        Some(kind) => kind,
        None => match accept {
            Some(accept) => {
                let parts = accept.split(',');
                for accepted in parts {
                    if let Some(kind) = ImageKind::from_content_type(accepted) {
                        return kind;
                    }
                }

                bucket.cfg()
                    .default_serving_format
                    .unwrap_or_else(|| bucket.cfg().formats.first_enabled_format())
            },
            None => bucket.cfg()
                .default_serving_format
                .unwrap_or_else(|| bucket.cfg().formats.first_enabled_format())
        },
    }
}


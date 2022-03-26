use bytes::Bytes;
use hashbrown::{HashMap, HashSet};
use poem_openapi::OpenApi;
use poem::{Body, Result};
use poem::web::headers::ContentType;
use poem_openapi::{Object, ApiResponse};
use poem_openapi::param::{Header, Path};
use poem_openapi::payload::{Binary, Json};
use uuid::Uuid;
use futures::StreamExt;

use crate::config::{config, RuntimeConfig, ImageKind};
use crate::controller::{BucketController, UploadInfo};


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
    /// Bucket not found
    #[oai(status = 401)]
    Unauthorized,
}


pub struct LustApi {
    pub buckets: HashMap<String, BucketController>,
}

#[OpenApi(prefix_path = "/:bucket")]
impl LustApi {
    /// Upload Image
    ///
    /// Upload an image to the given bucket.
    #[oai(path = "/", method = "post")]
    pub async fn upload_image(
        &self,
        bucket: Path<String>,
        #[oai(name = "content-length")] content_length: Header<Option<usize>>,
        #[oai(name = "content-type")] content_type: Header<String>,
        file: Binary<Body>,
    ) -> Result<UploadResponse> {
        let bucket = match self.buckets.get(&*bucket) {
            None => return Ok(UploadResponse::NotFound),
            Some(b) => b,
        };

        let format: ImageKind = match serde_json::from_str(&*content_type) {
            Err(_) => return Ok(UploadResponse::InvalidContentType),
            Ok(f) => f,
        };

        let length = match *content_length {
            None => return Ok(UploadResponse::MissingHeader),
            Some(length) =>  if !config().valid_global_size(length) {
                return Ok(UploadResponse::TooBig)
            } else {
                let local_limit = bucket
                    .cfg()
                    .max_upload_size
                    .map(|v| v as usize)
                    .unwrap_or(u32::MAX as usize);

                if length > local_limit  {
                    return Ok(UploadResponse::TooBig)
                }

                length
            }
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
}

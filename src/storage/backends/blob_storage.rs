use std::time::Duration;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use s3::{Bucket, Region};
use s3::creds::Credentials;
use uuid::Uuid;

use crate::config::ImageKind;
use crate::controller::get_bucket_by_id;
use crate::StorageBackend;

pub struct BlobStorageBackend {
    bucket: Bucket,
}

impl BlobStorageBackend {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        region: String,
        endpoint: String,
        access_key: Option<&str>,
        secret_key: Option<&str>,
        security_token: Option<&str>,
        session_token: Option<&str>,
        request_timeout: Option<Duration>,
    ) -> Result<Self> {
        let creds = Credentials::new(access_key, secret_key, security_token, session_token, None)?;
        let region = Region::Custom { region, endpoint };
        let mut bucket = Bucket::new(&name, region, creds)?;
        bucket.set_request_timeout(request_timeout);

        Ok(Self {
            bucket
        })
    }

    #[inline]
    fn format_path(
        &self,
        bucket_id: u32,
        sizing_id: u32,
        image_id: Uuid,
        format: ImageKind,
    ) -> String {
        format!("{}/{}/{}.{}", bucket_id, sizing_id, image_id, format.as_file_extension())
    }
}

#[async_trait]
impl StorageBackend for BlobStorageBackend {
    async fn store(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
        data: Bytes,
    ) -> anyhow::Result<()> {
        let store_in = self.format_path(bucket_id, sizing_id, image_id, kind);

        debug!("Storing image in bucket @ {}", &store_in);
        let (_, code) = self.bucket.put_object(store_in, &data).await?;
        if code != 200 {
            Err(anyhow!("Remote storage bucket did not respond correctly, expected status 200 got {}", code))
        } else {
            Ok(())
        }
    }

    async fn fetch(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
    ) -> anyhow::Result<Option<Bytes>> {
        let store_in = self.format_path(bucket_id, sizing_id, image_id, kind);

        debug!("Retrieving image in bucket @ {}", &store_in);
        let (data, code) = self.bucket.get_object(store_in).await?;
        if code == 404 {
            Ok(None)
        } else if code != 200 {
            Err(anyhow!("Remote storage bucket did not respond correctly, expected status 200 got {}", code))
        } else {
            Ok(Some(data.into()))
        }
    }

    async fn delete(
        &self,
        bucket_id: u32,
        image_id: Uuid,
    ) -> anyhow::Result<()> {
        let bucket = get_bucket_by_id(bucket_id)
            .ok_or_else(|| anyhow!("Bucket does not exist."))?
            .cfg();

        for sizing_id in bucket.sizing_preset_ids().iter().copied() {
            for kind in ImageKind::variants() {
                let store_in = self.format_path(bucket_id, sizing_id, image_id, *kind);

                debug!("Purging file in bucket @ {}", &store_in);
                let (_, code) = self.bucket.delete_object(store_in).await?;
                if code != 200 && code != 404 {
                    return Err(anyhow!(
                        "Remote storage bucket did not respond correctly, \
                        expected status 200 got {}", code
                    ))
                }
            }
        }

        Ok(())
    }
}




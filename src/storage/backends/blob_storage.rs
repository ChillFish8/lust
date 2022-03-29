use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use rusoto_core::credential::{AutoRefreshingProvider, ChainProvider};
use rusoto_core::{HttpClient, HttpConfig, Region};
use rusoto_s3::{DeleteObjectRequest, GetObjectRequest, PutObjectRequest, S3Client, S3, StreamingBody};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::config::ImageKind;
use crate::controller::get_bucket_by_id;
use crate::StorageBackend;

/// A credential timeout.
const CREDENTIAL_TIMEOUT: u64 = 5;

pub struct BlobStorageBackend {
    bucket_name: String,
    client: S3Client,
    store_public: bool,
}

impl BlobStorageBackend {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        region: String,
        endpoint: String,
        store_public: bool,
    ) -> Result<Self> {
        let mut chain_provider = ChainProvider::new();
        chain_provider.set_timeout(Duration::from_secs(CREDENTIAL_TIMEOUT));

        let credentials_provider = AutoRefreshingProvider::new(chain_provider)
            .with_context(|| "Failed to fetch credentials for the object storage.")?;

        let mut http_config: HttpConfig = HttpConfig::default();
        http_config.pool_idle_timeout(std::time::Duration::from_secs(10));

        let http_client = HttpClient::new_with_config(http_config)
            .with_context(|| "Failed to create request dispatcher")?;

        let region = Region::Custom { name: region, endpoint };

        let client = S3Client::new_with(
            http_client,
            credentials_provider,
            region,
        );

        Ok(Self {
            bucket_name: name,
            client,
            store_public,
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

        let request = PutObjectRequest {
            bucket: self.bucket_name.clone(),
            key: store_in,
            body: Some(StreamingBody::from(data.to_vec())),
            content_length: Some(data.len() as i64),
            acl: if self.store_public { Some("public-read".to_string()) } else { None },
            ..Default::default()
        };

        self.client.put_object(request).await?;
        Ok(())
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
        let request = GetObjectRequest {
            key: store_in,
            bucket: self.bucket_name.clone(),
            ..Default::default()
        };
        let res = self.client.get_object(request).await?;
        let content_length = res.content_length.unwrap_or(0) as usize;

        if let Some(body) = res.body {
            let mut buffer = Vec::with_capacity(content_length);
            body
                .into_async_read()
                .read_to_end(&mut buffer)
                .await?;

            Ok(Some(buffer.into()))
        } else {
            Ok(None)
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
                let request = DeleteObjectRequest {
                    bucket: self.bucket_name.clone(),
                    key: store_in,
                    ..Default::default()
                };
                self.client.delete_object(request).await?;
            }
        }

        Ok(())
    }
}




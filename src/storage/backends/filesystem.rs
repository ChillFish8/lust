use std::io::ErrorKind;
use std::path::PathBuf;
use anyhow::anyhow;
use async_trait::async_trait;
use bytes::Bytes;
use uuid::Uuid;

use crate::config::ImageKind;
use crate::controller::get_bucket_by_id;
use crate::StorageBackend;

pub struct FileSystemBackend {
    directory: PathBuf,
}

impl FileSystemBackend {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            directory: dir,
        }
    }

    #[inline]
    fn format_path(&self, bucket_id: u32, sizing_id: u32) -> PathBuf {
        self.directory
            .join(bucket_id.to_string())
            .join(sizing_id.to_string())
    }
}

#[async_trait]
impl StorageBackend for FileSystemBackend {
    async fn store(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
        data: Bytes,
    ) -> anyhow::Result<()> {
        let store_in = self.format_path(bucket_id, sizing_id);
        let path = store_in.join(format!("{}.{}", image_id, kind.as_file_extension()));

        debug!("Storing image @ {:?}", &path);
        match tokio::fs::write(&path, &data).await {
            Ok(()) => Ok(()),
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                tokio::fs::create_dir_all(store_in).await?;
                tokio::fs::write(&path, data).await?;
                Ok(())
            },
            Err(other) => Err(other.into())
        }
    }

    async fn fetch(
        &self,
        bucket_id: u32,
        image_id: Uuid,
        kind: ImageKind,
        sizing_id: u32,
    ) -> anyhow::Result<Option<Bytes>> {
        let store_in = self.format_path(bucket_id, sizing_id);
        let path = store_in.join(format!("{}.{}", image_id, kind.as_file_extension()));

        debug!("Retrieving image  @ {:?}", &path);
        match tokio::fs::read(&path).await {
            Ok(data) => Ok(Some(Bytes::from(data))),
            Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(other) => Err(other.into()),
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
                let store_in = self.format_path(bucket_id, sizing_id);
                let path = store_in.join(format!("{}.{}", image_id, kind.as_file_extension()));
                debug!("Purging image  @ {:?}", &path);

                 match tokio::fs::remove_file(&path).await {
                    Ok(()) => continue,
                    Err(ref e) if e.kind() == ErrorKind::NotFound => continue,
                    Err(other) => return Err(other.into()),
                }
            }
        }

        Ok(())
    }
}




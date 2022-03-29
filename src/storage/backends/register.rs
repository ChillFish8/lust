use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use serde::Deserialize;

use crate::StorageBackend;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendConfigs {
    // Scylla {
    //     nodes: Vec<String>,
    // },
    FileSystem {
        /// The base output directory to store files.
        directory: PathBuf,
    },
    BlobStorage {
        /// The name of the bucket.
        name: String,

        /// The region of the bucket.
        region: String,

        /// The bucket endpoint.
        endpoint: String,

        /// The optional bucket access_key.
        access_key: Option<String>,

        /// The optional bucket secret key.
        secret_key: Option<String>,

        /// The optional bucket security token.
        security_token: Option<String>,

        /// The optional bucket session token.
        session_token: Option<String>,

        /// A optional request timeout in seconds.
        request_timeout: Option<u32>,
    }
}

impl BackendConfigs {
    pub async fn connect(&self) -> anyhow::Result<Arc<dyn StorageBackend>> {
        match self {
            Self::FileSystem { directory } => {
                Ok(Arc::new(super::filesystem::FileSystemBackend::new(directory.clone())))
            },
            Self::BlobStorage {
                name,
                region,
                endpoint,
                access_key,
                secret_key,
                security_token,
                session_token,
                request_timeout,
            } => {
                let timeout = request_timeout.map(|v| Duration::from_secs(v as u64));
                let backend = super::blob_storage::BlobStorageBackend::new(
                    name.to_string(),
                    region.to_string(),
                    endpoint.to_string(),
                    access_key.as_ref().map(|v| v.as_str()),
                    secret_key.as_ref().map(|v| v.as_str()),
                    security_token.as_ref().map(|v| v.as_str()),
                    session_token.as_ref().map(|v| v.as_str()),
                    timeout,
                )?;

                Ok(Arc::new(backend))
            },
        }
    }
}

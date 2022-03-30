use std::path::PathBuf;
use std::sync::Arc;
use serde::Deserialize;

use crate::StorageBackend;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendConfigs {
    Scylla {
        nodes: Vec<String>,
        username: Option<String>,
        password: Option<String>,
        keyspace: String,
        table: Option<String>,
    },
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

        #[serde(default)]
        /// Store objects with the `public-read` acl.
        store_public: bool,
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
                store_public,
            } => {
                let backend = super::blob_storage::BlobStorageBackend::new(
                    name.to_string(),
                    region.to_string(),
                    endpoint.to_string(),
                    *store_public,
                )?;

                Ok(Arc::new(backend))
            },
            Self::Scylla {
                nodes,
                username,
                password,
                keyspace,
                table,
            } => {
                let backend = super::scylladb::ScyllaBackend::connect(
                    keyspace.clone(),
                    table.clone(),
                    nodes,
                    username.clone(),
                    password.clone(),
                ).await?;

                Ok(Arc::new(backend))
            }
        }
    }
}

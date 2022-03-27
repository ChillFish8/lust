use std::path::PathBuf;
use std::sync::Arc;
use serde::Deserialize;

use crate::StorageBackend;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendConfigs {
    // Scylla {
    //     nodes: Vec<String>,
    // },
    FileSystem {
        directory: PathBuf,
    }
}

impl BackendConfigs {
    pub async fn connect(&self) -> anyhow::Result<Arc<dyn StorageBackend>> {
        match self {
            Self::FileSystem { directory } => {
                Ok(Arc::new(super::filesystem::FileSystemBackend::new(directory.clone())))
            }
        }
    }
}

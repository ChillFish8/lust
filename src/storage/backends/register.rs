use std::sync::Arc;
use serde::Deserialize;
use crate::StorageBackend;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendConfigs {
    Scylla {
        nodes: Vec<String>,
    }
}

impl BackendConfigs {
    pub async fn connect(&self) -> anyhow::Result<Arc<dyn StorageBackend>> {
        todo!()
    }
}

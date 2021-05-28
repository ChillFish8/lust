use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use gotham_derive::StateData;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use uuid::Uuid;

use crate::backends;
use crate::context::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};

// The bellow definitions are a hack, this is due to
pub(crate) static CASSANDRA: OnceCell<backends::cql::Backend> = OnceCell::new();
pub(crate) static POSTGRES: OnceCell<backends::sql::PostgresBackend> = OnceCell::new();
pub(crate) static MYSQL: OnceCell<backends::sql::MySQLBackend> = OnceCell::new();
pub(crate) static SQLITE: OnceCell<backends::sql::SqliteBackend> = OnceCell::new();

#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "config")]
pub enum DatabaseBackend {
    Cassandra(backends::cql::DatabaseConfig),
    Postgres(backends::sql::DatabaseConfig),
    MySQL(backends::sql::DatabaseConfig),
    Sqlite(backends::sql::DatabaseConfig),
}

macro_rules! acquire {
    ( $e:expr ) => {{
        $e.get().expect("backend not initialised")
    }};
}

#[derive(Copy, Clone)]
pub enum Backend {
    Cassandra,
    Postgres,
    MySQL,
    Sqlite,
}

#[async_trait]
impl DatabaseLinker for Backend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).ensure_tables(presets, formats).await,
            Self::Postgres => acquire!(POSTGRES).ensure_tables(presets, formats).await,
            Self::MySQL => acquire!(MYSQL).ensure_tables(presets, formats).await,
            Self::Sqlite => acquire!(SQLITE).ensure_tables(presets, formats).await,
        }
    }
}

#[async_trait]
impl ImageStore for Backend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).get_image(file_id, preset, format).await,
            Self::Postgres => acquire!(POSTGRES).get_image(file_id, preset, format).await,
            Self::MySQL => acquire!(MYSQL).get_image(file_id, preset, format).await,
            Self::Sqlite => acquire!(SQLITE).get_image(file_id, preset, format).await,
        }
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).add_image(file_id, data).await,
            Self::Postgres => acquire!(POSTGRES).add_image(file_id, data).await,
            Self::MySQL => acquire!(MYSQL).add_image(file_id, data).await,
            Self::Sqlite => acquire!(SQLITE).add_image(file_id, data).await,
        }
    }

    async fn remove_image(&self, file_id: Uuid) -> Result<()> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).remove_image(file_id).await,
            Self::Postgres => acquire!(POSTGRES).remove_image(file_id).await,
            Self::MySQL => acquire!(MYSQL).remove_image(file_id).await,
            Self::Sqlite => acquire!(SQLITE).remove_image(file_id).await,
        }
    }
}

#[derive(Copy, Clone, StateData)]
pub struct StorageBackend(Backend);

impl StorageBackend {
    pub fn with_backend(backend: Backend) -> Self {
        Self { 0: backend }
    }
}

#[async_trait]
impl DatabaseLinker for StorageBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        self.0.ensure_tables(presets, formats).await
    }
}

#[async_trait]
impl ImageStore for StorageBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        self.0.get_image(file_id, preset, format).await
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        self.0.add_image(file_id, data).await
    }

    async fn remove_image(&self, file_id: Uuid) -> Result<()> {
        self.0.remove_image(file_id).await
    }
}

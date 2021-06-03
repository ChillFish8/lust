use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use gotham_derive::StateData;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use uuid::Uuid;

use crate::backends;
use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
use crate::traits::ImageStore;

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

#[derive(Copy, Clone, StateData)]
pub enum StorageBackend {
    Cassandra,
    Postgres,
    MySQL,
    Sqlite,
}

#[async_trait]
impl ImageStore for StorageBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        match self {
            Self::Cassandra => {
                acquire!(CASSANDRA)
                    .get_image(file_id, preset, category, format)
                    .await
            }
            Self::Postgres => {
                acquire!(POSTGRES)
                    .get_image(file_id, preset, category, format)
                    .await
            }
            Self::MySQL => {
                acquire!(MYSQL)
                    .get_image(file_id, preset, category, format)
                    .await
            }
            Self::Sqlite => {
                acquire!(SQLITE)
                    .get_image(file_id, preset, category, format)
                    .await
            }
        }
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).add_image(file_id, category, data).await,
            Self::Postgres => acquire!(POSTGRES).add_image(file_id, category, data).await,
            Self::MySQL => acquire!(MYSQL).add_image(file_id, category, data).await,
            Self::Sqlite => acquire!(SQLITE).add_image(file_id, category, data).await,
        }
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).remove_image(file_id, presets).await,
            Self::Postgres => acquire!(POSTGRES).remove_image(file_id, presets).await,
            Self::MySQL => acquire!(MYSQL).remove_image(file_id, presets).await,
            Self::Sqlite => acquire!(SQLITE).remove_image(file_id, presets).await,
        }
    }

    async fn list_entities(
        &self,
        filter: FilterType,
        order: OrderBy,
        page: usize,
    ) -> Result<Vec<IndexResult>> {
        match self {
            Self::Cassandra => acquire!(CASSANDRA).list_entities(filter, order, page).await,
            Self::Postgres => acquire!(POSTGRES).list_entities(filter, order, page).await,
            Self::MySQL => acquire!(MYSQL).list_entities(filter, order, page).await,
            Self::Sqlite => acquire!(SQLITE).list_entities(filter, order, page).await,
        }
    }
}

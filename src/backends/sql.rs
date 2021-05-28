#![allow(unused)]

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use serde::Deserialize;
use serde_variant::to_variant_name;
use std::env::var;
use std::sync::Arc;
use uuid::Uuid;

use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Pool, Row};

use crate::context::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};

/// The configuration for the SQL based database backends.
///
/// The `connection_uri` should be formatted as a direct connect
/// uri. e.g.
/// `postgresql://john:boo@localhost/postgres`
///
/// The `pool_size` determined the *maximum* amount of pool connections.
#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    connection_uri: String,
    pool_size: u32,
}

fn select(column: &str, preset: &str, placeholder: &str) -> String {
    format!(
        "SELECT {column} FROM {table} WHERE file_id = {placeholder} LIMIT 1;",
        column = column,
        table = preset,
        placeholder = placeholder,
    )
}

/// A database backend set to handle the PostgreSQL database.
pub struct PostgresBackend {
    cfg: DatabaseConfig,
    pool: PgPool,
}

impl PostgresBackend {
    /// Connect to the given PostgreSQL server.
    ///
    /// This will build a connection pool and connect with a maximum
    /// of n connections determined by the `pool_size` of the given
    /// config.
    pub async fn connect(cfg: DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(cfg.pool_size)
            .connect(&cfg.connection_uri)
            .await?;

        Ok(Self { cfg, pool })
    }
}

#[async_trait]
impl DatabaseLinker for PostgresBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} BYTEA", column))
        }

        for preset in presets {
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let qry = sqlx::query(&query);

            qry.execute(&self.pool).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl ImageStore for PostgresBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        let column = to_variant_name(&format).expect("unreachable");

        let qry = select(column, &preset, "$1");
        let qry = sqlx::query(&qry).bind(file_id.to_string());

        if let Ok(row) = qry.fetch_one(&self.pool).await {
            let data: &[u8] = row.get(column);
            Some(BytesMut::from(data))
        } else {
            None
        }
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid) -> Result<()> {
        unimplemented!()
    }
}

/// A database backend set to handle the MySQL / MariaDB database.
pub struct MySQLBackend {
    cfg: DatabaseConfig,
    pool: MySqlPool,
}

impl MySQLBackend {
    /// Connect to the given MySQL / MariaDB server.
    ///
    /// This will build a connection pool and connect with a maximum
    /// of n connections determined by the `pool_size` of the given
    /// config.
    pub async fn connect(cfg: DatabaseConfig) -> Result<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(cfg.pool_size)
            .connect(&cfg.connection_uri)
            .await?;

        Ok(Self { cfg, pool })
    }
}

#[async_trait]
impl DatabaseLinker for MySQLBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} LONGBLOB", column))
        }

        for preset in presets {
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let qry = sqlx::query(&query);

            qry.execute(&self.pool).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl ImageStore for MySQLBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        let column = to_variant_name(&format).expect("unreachable");

        let qry = select(column, &preset, "%s");
        let qry = sqlx::query(&qry).bind(file_id.to_string());

        if let Ok(row) = qry.fetch_one(&self.pool).await {
            let data: &[u8] = row.get(column);
            Some(BytesMut::from(data))
        } else {
            None
        }
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid) -> Result<()> {
        unimplemented!()
    }
}

/// A database backend set to handle the Sqlite database.
///
/// Due to the nature of SQLite this is *not* recommended for use
/// in production being a single file. Consider using something like
/// PostgreSQL or Cassandra in production.
///
/// This backend requires that the system uses a standard File approach e.g.
/// not im memory / shared memory due to the sqlx::Pool handling.
/// If in-memory is used this can produce undefined behaviour in terms
/// of what data is perceived to be stored.
pub struct SqliteBackend {
    cfg: DatabaseConfig,
    pool: SqlitePool,
}

impl SqliteBackend {
    /// Connect to the given Sqlite file.
    ///
    /// This will build a connection pool and connect with a maximum
    /// of n connections determined by the `pool_size` of the given
    /// config.
    ///
    /// Due to the nature of this being a pool setup, in-memory setups are
    /// not supported.
    pub async fn connect(cfg: DatabaseConfig) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(cfg.pool_size)
            .connect(&cfg.connection_uri)
            .await?;

        Ok(Self { cfg, pool })
    }
}

#[async_trait]
impl DatabaseLinker for SqliteBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} BLOB", column))
        }

        for preset in presets {
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let qry = sqlx::query(&query);

            qry.execute(&self.pool).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl ImageStore for SqliteBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        let column = to_variant_name(&format).expect("unreachable");

        let qry = select(column, &preset, "?");
        let qry = sqlx::query(&qry).bind(file_id.to_string());

        if let Ok(row) = qry.fetch_one(&self.pool).await {
            let data: &[u8] = row.get(column);
            Some(BytesMut::from(data))
        } else {
            None
        }
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid) -> Result<()> {
        unimplemented!()
    }
}

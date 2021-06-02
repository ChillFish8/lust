use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use chrono::Utc;
use log::{debug, info};
use serde::Deserialize;
use serde_variant::to_variant_name;
use std::str::FromStr;
use uuid::Uuid;

use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;

use crate::configure::PAGE_SIZE;
use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
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

fn build_select_qry(column: &str, preset: &str, placeholder: &str) -> String {
    format!(
        "SELECT {column} FROM {table} WHERE file_id = {placeholder} LIMIT 1;",
        column = column,
        table = preset,
        placeholder = placeholder,
    )
}

fn build_insert_qry(preset: &str, columns: &Vec<&str>, placeholders: &Vec<String>) -> String {
    let columns = columns.join(", ");
    let placeholders = placeholders.join(", ");
    format!(
        "INSERT INTO {table} ({columns}) VALUES ({placeholders});",
        table = preset,
        columns = columns,
        placeholders = placeholders,
    )
}

fn build_delete_queries(presets: &Vec<&String>, placeholder: &str) -> Vec<String> {
    let mut queries = vec![];
    for preset in presets {
        queries.push(format!(
            "DELETE FROM {table} WHERE file_id = {placeholder};",
            table = preset,
            placeholder = placeholder,
        ))
    }

    queries
}

/// Either extracts the value as a `&[u8]` from the row as `Some(BytesMut)`
/// or becomes `None`.
macro_rules! extract_or_none {
    ( $e:expr, $c:expr ) => {{
        if let Ok(row) = $e {
            let data: &[u8] = row.get($c);
            Some(BytesMut::from(data))
        } else {
            None
        }
    }};
}

/// Builds a SQL query for the given preset (table) from
/// the given data adding place holders for each value for
/// prepared statements.
macro_rules! build_insert {
    ( $preset:expr, $data:expr, $placeholder:expr ) => {{
        let mut columns: Vec<&str> = $data
            .keys()
            .map(|v| to_variant_name(v).expect("unreachable"))
            .collect();
        columns.insert(0, "file_id");

        let values: Vec<BytesMut> = $data.values().map(|v| v.clone()).collect();

        let placeholders: Vec<String> = (1..columns.len() + 1).map($placeholder).collect();

        (build_insert_qry($preset, &columns, &placeholders), values)
    }};
}

/// Builds a sqlx query based on the given query string and values
///
/// This also accounts for the file_id being a uuid vs everything else
/// being bytes.
macro_rules! query_with_parameters {
    ( $id:expr, $qry:expr, $values:expr ) => {{
        let mut qry = sqlx::query($qry).bind($id);

        for value in $values {
            qry = qry.bind(value)
        }

        qry
    }};
}

/// Deletes a file with a given id from all presets.
///
/// Due to the nature of the Pool types but the similarity between
/// each database code to delete files it makes more sense to put this
/// in a macro over a function.
macro_rules! delete_file {
    ( $id:expr, $presets:expr, $placeholder:expr, $pool:expr ) => {{
        let file_id = $id.to_string();
        let queries = build_delete_queries($presets, $placeholder);

        for qry in queries {
            let query = sqlx::query(&qry).bind(&file_id);
            query.execute($pool).await?;
        }

        let qry = format!(
            "DELETE FROM image_metadata WHERE file_id = {}",
            $placeholder,
        );

        let query = sqlx::query(&qry).bind($id.to_string());
        query.execute($pool).await?;
    }};
}

/// Inserts a given file_id into the index table.
///
/// This table mostly acts as the metadata table for listing files of
/// given categories.
macro_rules! insert_metadata {
    ( $file_id:expr, $category:expr, $total:expr, $placeholder:expr, $pool:expr, ) => {{
        let placeholders: String = (0..4).map($placeholder).collect::<Vec<String>>().join(", ");

        let qry = format!(
            r#"
        INSERT INTO image_metadata (
            file_id,
            category,
            insert_date,
            total_size
        ) VALUES ({placeholders})"#,
            placeholders = placeholders,
        );

        let now = Utc::now();

        let query = sqlx::query(&qry)
            .bind($file_id)
            .bind($category)
            .bind(now)
            .bind($total);
        query.execute($pool).await?;
    }};
}

macro_rules! sum_total {
    ( $total:expr, $values:expr ) => {{
        let sum: i64 = $values.values().map(|v| v.len() as i64).sum();
        $total += sum;
    }};
}

macro_rules! check_category {
    ( $file_id:expr, $category:expr, $ph1:expr, $ph2:expr, $pool:expr ) => {{
        let qry = format!(
            "SELECT 1 FROM image_metadata WHERE file_id = {} AND category = {};",
            $ph1, $ph2,
        );

        sqlx::query(&qry)
            .bind($file_id.to_string())
            .bind($category)
            .fetch_optional($pool)
            .await
            .unwrap_or(None)
    }};
}

macro_rules! apply_filter {
    ( $qry:expr, $placeholder:expr, $filter:expr ) => {{
        match $filter {
            FilterType::All => (),
            FilterType::Category(_) => $qry = format!("{} WHERE category = {}", $qry, $placeholder),
            FilterType::CreationDate(_) => {
                $qry = format!("{} WHERE insert_date = {}", $qry, $placeholder)
            }
        };
    }};
}

macro_rules! bind_filter {
    ( $query:expr, $filter:expr ) => {{
        match $filter {
            FilterType::All => (),
            FilterType::Category(v) => $query = $query.bind(v),
            FilterType::CreationDate(v) => $query = $query.bind(v),
        };
    }};
}

macro_rules! from_rows {
    ( $rows:expr ) => {{
        $rows
            .drain(..)
            .map(|v| IndexResult {
                file_id: Uuid::from_str(v.get("file_id")).expect("uuid was invalid in database"),
                category: v.get("category"),
                total_size: v.get("total_size"),
                created_on: v.get("insert_date"),
            })
            .collect()
    }};
}

/// A database backend set to handle the PostgreSQL database.
pub struct PostgresBackend {
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

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseLinker for PostgresBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        info!("building tables");

        let query = sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS image_metadata (
            file_id CHAR(36) PRIMARY KEY,
            category TEXT,
            insert_date TIMESTAMP,
            total_size INTEGER
        )"#,
        );

        query.execute(&self.pool).await?;

        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} BYTEA", column))
        }

        for preset in presets {
            let qry = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let query = sqlx::query(&qry);

            query.execute(&self.pool).await?;
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
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        check_category!(file_id, category, "$1", "$2", &self.pool)?;

        let column = to_variant_name(&format).expect("unreachable");

        let qry = build_select_qry(column, &preset, "$1");
        let qry = sqlx::query(&qry).bind(file_id.to_string());

        extract_or_none!(qry.fetch_one(&self.pool).await, column)
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        let mut total: i64 = 0;
        for (preset, preset_data) in data {
            sum_total!(total, preset_data);
            let (qry, values) = build_insert!(&preset, preset_data, |i| format!("${}", i));

            let values_ = values.iter().map(|v| v.as_ref());
            let query = query_with_parameters!(file_id.to_string(), &qry, values_);
            query.execute(&self.pool).await?;
        }

        insert_metadata!(
            file_id.to_string(),
            category,
            total,
            |i| format!("${}", i),
            &self.pool,
        );

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        delete_file!(file_id, &presets, "$1", &self.pool);

        Ok(())
    }

    async fn list_entities(
        &self,
        filter: FilterType,
        order: OrderBy,
        page: usize,
    ) -> Result<Vec<IndexResult>> {
        // we start at 1 but the offset should be calculated from 0
        let skip = PAGE_SIZE * page as i64 - 1;
        let order = order.as_str();

        let mut qry = format!(
            r#"
            SELECT (file_id, category, insert_date, total_size)
            FROM image_metadata
            OFFSET $1
            LIMIT $2
            ORDER BY {}
            "#,
            order
        );

        apply_filter!(qry, "$3", &filter);

        let mut query = sqlx::query(&qry).bind(skip).bind(PAGE_SIZE);

        bind_filter!(query, filter);

        let mut rows = query.fetch_all(&self.pool).await?;
        let results = from_rows!(rows);

        Ok(results)
    }
}

/// A database backend set to handle the MySQL / MariaDB database.
pub struct MySQLBackend {
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

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseLinker for MySQLBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        info!("building tables");

        let query = sqlx::query(
            r#"
        CREATE TABLE IF NOT EXISTS image_metadata (
            file_id CHAR(36) PRIMARY KEY,
            category TEXT,
            insert_date TIMESTAMP,
            total_size INTEGER
        )"#,
        );

        query.execute(&self.pool).await?;

        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} LONGBLOB", column))
        }

        for preset in presets {
            let qry = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let query = sqlx::query(&qry);

            query.execute(&self.pool).await?;
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
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        check_category!(file_id, category, "?", "?", &self.pool)?;

        let column = to_variant_name(&format).expect("unreachable");

        let qry = build_select_qry(column, &preset, "?");
        let query = sqlx::query(&qry).bind(file_id.to_string());

        extract_or_none!(query.fetch_one(&self.pool).await, column)
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        let mut total: i64 = 0;
        for (preset, preset_data) in data {
            sum_total!(total, preset_data);
            let (qry, values) = build_insert!(&preset, preset_data, |_| "?".to_string());

            let values_ = values.iter().map(|v| v.as_ref());
            let query = query_with_parameters!(file_id.to_string(), &qry, values_);
            query.execute(&self.pool).await?;
        }

        insert_metadata!(
            file_id.to_string(),
            category,
            total,
            |_| "?".to_string(),
            &self.pool,
        );

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        delete_file!(file_id, &presets, "?", &self.pool);
        Ok(())
    }

    async fn list_entities(
        &self,
        filter: FilterType,
        order: OrderBy,
        page: usize,
    ) -> Result<Vec<IndexResult>> {
        // we start at 1 but the offset should be calculated from 0
        let skip = PAGE_SIZE * page as i64 - 1;
        let order = order.as_str();

        let mut qry = format!(
            r#"
            SELECT (file_id, category, insert_date, total_size)
            FROM image_metadata
            LIMIT ?, ?
            ORDER BY {}
            "#,
            order
        );

        apply_filter!(qry, "?", &filter);

        let mut query = sqlx::query(&qry).bind(skip).bind(PAGE_SIZE);

        bind_filter!(query, filter);

        let mut rows = query.fetch_all(&self.pool).await?;
        let results = from_rows!(rows);

        Ok(results)
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

        info!("successfully connected to sqlite");

        Ok(Self { pool })
    }
}

#[async_trait]
impl DatabaseLinker for SqliteBackend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        info!("building tables");

        let query = sqlx::query(
            "
        CREATE TABLE IF NOT EXISTS image_metadata (
            file_id CHAR(36) PRIMARY KEY,
            category TEXT,
            insert_date TEXT,
            total_size INTEGER
        )",
        );

        query.execute(&self.pool).await?;
        info!("metadata table created successfully");

        let mut columns = vec![format!("file_id CHAR(36) PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} BLOB", column))
        }

        for preset in presets {
            let qry = format!(
                "CREATE TABLE IF NOT EXISTS {table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            let query = sqlx::query(&qry);

            query.execute(&self.pool).await?;

            debug!("created preset table {}", preset);
        }
        info!("all preset tables created successfully");

        Ok(())
    }
}

#[async_trait]
impl ImageStore for SqliteBackend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        check_category!(file_id, category, "?", "?", &self.pool)?;

        let column = to_variant_name(&format).expect("unreachable");

        let qry = build_select_qry(column, &preset, "?");
        let query = sqlx::query(&qry).bind(file_id.to_string());

        extract_or_none!(query.fetch_one(&self.pool).await, column)
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        let mut total: i64 = 0;
        for (preset, preset_data) in data {
            sum_total!(total, preset_data);

            let (qry, values) = build_insert!(&preset, preset_data, |_| "?".to_string());

            let values_ = values.iter().map(|v| v.as_ref());
            let query = query_with_parameters!(file_id.to_string(), &qry, values_);
            query.execute(&self.pool).await?;
        }

        insert_metadata!(
            file_id.to_string(),
            category,
            total,
            |_| "?".to_string(),
            &self.pool,
        );

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        delete_file!(file_id, &presets, "?", &self.pool);
        Ok(())
    }

    async fn list_entities(
        &self,
        filter: FilterType,
        order: OrderBy,
        page: usize,
    ) -> Result<Vec<IndexResult>> {
        // we start at 1 but the offset should be calculated from 0
        let skip = PAGE_SIZE * (page as i64 - 1);
        let order = match order {
            OrderBy::CreationDate => "datetime(insert_date)",
            OrderBy::TotalSize => "total_size",
        };

        let mut qry = format!(
            r#"
            SELECT file_id, category, insert_date, total_size
            FROM image_metadata
            ORDER BY {} DESC
            LIMIT ?, ?;
            "#,
            order
        );

        apply_filter!(qry, "?", &filter);

        let mut query = sqlx::query(&qry).bind(skip).bind(PAGE_SIZE);

        bind_filter!(query, filter);

        let mut rows = query.fetch_all(&self.pool).await?;
        let results = from_rows!(rows);

        Ok(results)
    }
}

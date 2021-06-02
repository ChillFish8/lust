use scylla::transport::session::Session;
use scylla::SessionBuilder;
use scylla::query::Query;

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use chrono::Utc;
use log::{debug, info, warn};
use serde::Deserialize;
use serde_variant::to_variant_name;
use uuid::Uuid;

use crate::context::{FilterType, IndexResult, OrderBy};
use crate::image::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};
use crate::configure::PAGE_SIZE;

/// Represents a connection pool session with a round robbin load balancer.
type CurrentSession = Session;

/// The configuration for a cassandra database.
///
/// Each cluster should be given in the `host:port` format and
/// should only be the main node (not replication nodes).
///
/// The replication_factor is used when the keyspace is first created,
/// if the keyspace already exists this number may be ignored despite
/// being changed due to current implementation limitations.
///
/// The replication_class is used when the keyspace is first created,
/// this has the same caveats as the replication_factor.
#[derive(Clone, Deserialize)]
pub struct DatabaseConfig {
    clusters: Vec<String>,
    replication_factor: usize,
    replication_class: String,
    user: String,
    password: String,
}

macro_rules! log_and_convert_error {
    ( $e:expr ) => {{
        match $e {
            Ok(frame) => Some(frame),
            Err(e) => {
                warn!("failed to execute query {:?}", e);
                None
            }
        }
    }};
}

/// A cassandra database backend.
pub struct Backend {
    session: CurrentSession,
}

impl Backend {
    pub async fn connect(cfg: DatabaseConfig) -> Result<Self> {
        info!("connecting to database");
        let session = SessionBuilder::new()
            .user(cfg.user, cfg.password)
            .known_nodes(cfg.clusters.as_ref())
            .build()
            .await?;
        info!("connect successful");

        let create_ks = format!(
            r#"
        CREATE KEYSPACE IF NOT EXISTS lust_ks WITH REPLICATION  = {{
            'class': '{}', 'replication_factor': {}
        }};"#,
            &cfg.replication_class, cfg.replication_factor
        );
        debug!("creating keyspace {}", &create_ks);

        let _ = session.query(create_ks, &[]).await?;
        info!("keyspace ensured");

        Ok(Self { session })
    }
}

#[async_trait]
impl DatabaseLinker for Backend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        info!("building tables");
        let query = r#"
        CREATE TABLE IF NOT EXISTS lust_ks.image_metadata (
            file_id UUID PRIMARY KEY,
            category TEXT,
            insert_date TIMESTAMP,
            total_size INTEGER
        );"#;

        self.session.query(query, &[]).await?;
        info!("metadata table created successfully");

        let mut columns = vec![format!("file_id UUID PRIMARY KEY")];

        for format in formats {
            let column = to_variant_name(&format).expect("unreachable");
            columns.push(format!("{} BLOB", column))
        }

        for preset in presets {
            let query = format!(
                "CREATE TABLE IF NOT EXISTS lust_ks.{table} ({columns})",
                table = preset,
                columns = columns.join(", ")
            );

            self.session.query(query, &[]).await?;

            debug!("created preset table {}", preset);
        }
        info!("tables created");

        Ok(())
    }
}

#[async_trait]
impl ImageStore for Backend {
    async fn get_image(
        &self,
        file_id: Uuid,
        preset: String,
        category: &str,
        format: ImageFormat,
    ) -> Option<BytesMut> {
        let qry = r#"
        SELECT 1 FROM lust_ks.image_metadata
        WHERE file_id = ? AND category = ?;
        "#;
        let prepared = log_and_convert_error!(self.session.prepare(qry,).await)?;

        let query_result =
            log_and_convert_error!(self.session.execute(&prepared, (file_id,)).await)?;

        let _ = query_result.rows?;

        let column = to_variant_name(&format).expect("unreachable");
        let qry = format!(
            "SELECT {column} FROM lust_ks.{table} WHERE file_id = ? LIMIT 1;",
            column = column,
            table = preset,
        );

        let prepared = log_and_convert_error!(self.session.prepare(qry,).await)?;

        let query_result =
            log_and_convert_error!(self.session.execute(&prepared, (file_id,)).await)?;

        let mut rows = query_result.rows?;
        let row = rows.pop()?;
        let (data,) = log_and_convert_error!(row.into_typed::<(Vec<u8>,)>())?;
        let ref_: &[u8] = data.as_ref();
        Some(BytesMut::from(ref_))
    }

    async fn add_image(&self, file_id: Uuid, category: &str, data: ImagePresetsData) -> Result<()> {
        let mut total: i64 = 0;
        for (preset, preset_data) in data {
            let sum: i64 = preset_data.values().map(|v| v.len() as i64).sum();
            total += sum;

            let columns: String = preset_data
                .keys()
                .map(|v| to_variant_name(v).expect("unreachable"))
                .collect::<Vec<&str>>()
                .join(", ");

            let placeholders: String = (0..preset_data.len())
                .map(|_| "?")
                .collect::<Vec<&str>>()
                .join(", ");

            let mut values: Vec<Vec<u8>> = preset_data.values().map(|v| v.to_vec()).collect();

            values.insert(0, file_id.as_bytes().to_vec());

            let qry = format!(
                "INSERT INTO lust_ks.{table} (file_id, {columns}) VALUES (?, {placeholders});",
                table = preset,
                columns = columns,
                placeholders = placeholders,
            );

            let prepared = self.session.prepare(qry).await?;
            self.session.execute(&prepared, values).await?;
        }

        let qry = r#"
        INSERT INTO lust_ks.image_metadata (
            file_id,
            category,
            insert_date,
            total_size
        ) VALUES (?, ?, ?, ?);"#;

        let now = Utc::now();

        self.session
            .query(qry, (file_id, category, now.to_string(), total))
            .await?;
        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        for preset in presets {
            let qry = format!(
                "DELETE FROM lust_ks.{table} WHERE file_id = ?;",
                table = preset,
            );

            self.session
                .query(qry, (file_id.as_bytes().to_vec(),))
                .await?;
        }

        let qry = "DELETE FROM lust_ks.image_metadata WHERE file_id = ?;";

        self.session.query(qry, (file_id,)).await?;
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
        let order = order.as_str();

        let qry = format!(r#"
            SELECT file_id, category, insert_date, total_size
            FROM lust_ks.image_metadata
            ORDER BY {} DESC
            "#, order);

        let mut query = match filter {
            FilterType::All => {
                let qry = format!("{};", qry);
                Query::new(qry)
            },
            FilterType::CreationDate(v) => {
                let qry = format!("{} WHERE insert_date = ?;", qry);
                Query::new(qry)
            },
            FilterType::Category(v) => {
                let qry = format!("{} WHERE category = ?;", qry);
                Query::new(qry)
            },
        };

        query.set_page_size(PAGE_SIZE as i32);




        Ok(results)
    }
}

use scylla::cql_to_rust::FromRow;
use scylla::macros::FromRow;
use scylla::transport::session::{IntoTypedRows, Session};
use scylla::SessionBuilder;

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use log::warn;
use serde::Deserialize;
use serde_variant::to_variant_name;
use uuid::Uuid;

use crate::context::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};

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
        let session = SessionBuilder::new()
            .user(cfg.user, cfg.password)
            .known_nodes(cfg.clusters.as_ref())
            .build()
            .await?;

        let create_ks = format!(
            r#"
        CREATE KEYSPACE IF NOT EXISTS lust_ks WITH REPLICATION  = {{
            'class': {:?}, 'replication_factor': {}
        }};"#,
            &cfg.replication_class, cfg.replication_factor
        );

        let _ = session.query(create_ks, &[]).await?;

        Ok(Self { session })
    }
}

#[async_trait]
impl DatabaseLinker for Backend {
    async fn ensure_tables(&self, presets: Vec<&str>, formats: Vec<ImageFormat>) -> Result<()> {
        let mut columns = vec![format!("file_id uuid PRIMARY KEY")];

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

            self.session.query(query, &[]).await?;
        }

        Ok(())
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
        let column = to_variant_name(&format).expect("unreachable");
        let qry = format!(
            "SELECT {column} FROM {table} WHERE file_id = ? LIMIT 1;",
            column = column,
            table = preset,
        );

        let prepared = log_and_convert_error!(
            self.session.prepare(qry,).await
        )?;

        let query_result = log_and_convert_error!(
            self.session.execute(&prepared, (file_id,)).await
        )?;

        let mut rows = query_result.rows?;
        let row = rows.pop()?;
        let (data,) = log_and_convert_error!(row.into_typed::<(Vec<u8>,)>())?;
        let ref_: &[u8] = data.as_ref();
        Some(BytesMut::from(ref_))
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        unimplemented!()
    }
}

use cdrs::authenticators::StaticPasswordAuthenticator;
use cdrs::cluster::session::{new as new_session, Session};
use cdrs::cluster::{ClusterTcpConfig, NodeTcpConfigBuilder, TcpConnectionPool};
use cdrs::load_balancing::RoundRobin;
use cdrs::types::IntoRustByName;
use cdrs::types::blob::Blob;
use cdrs::types::value::Value;
use cdrs::query::*;

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use serde::Deserialize;
use serde_variant::to_variant_name;
use uuid::Uuid;
use log::warn;

use crate::context::{ImageFormat, ImagePresetsData};
use crate::traits::{DatabaseLinker, ImageStore};

/// Represents a connection pool session with a round robbin load balancer.
type CurrentSession = Session<RoundRobin<TcpConnectionPool<StaticPasswordAuthenticator>>>;

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
            },
        }
    }};
}

/// A cassandra database backend.
pub struct Backend {
    session: CurrentSession,
}

impl Backend {
    pub async fn connect(cfg: DatabaseConfig) -> Result<Self> {
        let mut nodes = Vec::new();
        for node in cfg.clusters.iter() {
            let node = NodeTcpConfigBuilder::new(
                node,
                StaticPasswordAuthenticator::new(cfg.user.clone(), cfg.password.clone()),
            )
            .build();

            nodes.push(node);
        }

        let cluster_config = ClusterTcpConfig(nodes);
        let session = new_session(&cluster_config, RoundRobin::new()).await?;

        let create_ks = format!(
            r#"
        CREATE KEYSPACE IF NOT EXISTS lust_ks WITH REPLICATION  = {{
            'class': {:?}, 'replication_factor': {}
        }};"#,
            &cfg.replication_class, cfg.replication_factor
        );

        let _ = session.query(create_ks).await?;

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

            self.session.query(query).await?;
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

        let values = query_values!(file_id);
        let res = self.session
            .query_with_values(qry, values).await;

        let res = log_and_convert_error!(res)?;
        let res = log_and_convert_error!(res.get_body())?;
        let rows = res.into_rows()?;
        let first = &rows[0];
        let value: Option<Blob> = log_and_convert_error!(first.get_by_name(column))?;
        let value: Vec<u8> = value?.into_vec();
        let ref_: &[u8] = value.as_ref();
        Some(BytesMut::from(ref_))
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        for (preset, preset_data) in data {
            let columns = preset_data
                .keys()
                .map(|v| to_variant_name(v).expect("unreachable"))
                .collect::<Vec<&str>>()
                .join(", ");

            let placeholders = (1..preset_data.len() + 1)
                .map(|_| "?")
                .collect::<Vec<&str>>()
                .join(", ");

            let mut values: Vec<Value> = preset_data
                .values()
                .map(|d| Value::new_normal(d.to_vec()))
                .collect();

            values.insert(
                0,
                Value::new_normal(file_id.as_bytes().to_vec())
            );

            let qry = format!(
                "INSERT INTO {table} (file_id, {columns}) VALUES (?, {placeholders})",
                table = preset,
                columns = columns,
                placeholders = placeholders,
            );

            let values = QueryValues::SimpleValues(values);
            let _ = self.session
                .query_with_values(qry, values).await?;
        }

        Ok(())
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        for preset in presets {
            let qry = format!(
                "DELETE FROM {table} WHERE file_id = ?;",
                table = preset,
            );

            let values = query_values!(file_id);
            let _ = self.session
                .query_with_values(qry, values).await?;
        }

        Ok(())
    }
}

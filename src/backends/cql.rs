#![allow(unused)]

use cdrs::authenticators::{Authenticator, NoneAuthenticator, StaticPasswordAuthenticator};
use cdrs::cluster::session::{new as new_session, Session};
use cdrs::cluster::{ClusterTcpConfig, NodeTcpConfigBuilder, TcpConnectionPool};
use cdrs::load_balancing::RoundRobin;
use cdrs::query::*;

use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use serde::Deserialize;
use serde_variant::to_variant_name;
use uuid::Uuid;

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

/// A cassandra database backend.
pub struct Backend {
    cfg: DatabaseConfig,
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

        Ok(Self { cfg, session })
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
        unimplemented!()
    }

    async fn add_image(&self, file_id: Uuid, data: ImagePresetsData) -> Result<()> {
        unimplemented!()
    }

    async fn remove_image(&self, file_id: Uuid, presets: Vec<&String>) -> Result<()> {
        unimplemented!()
    }
}

use anyhow::anyhow;
use bytes::Bytes;
use uuid::Uuid;
use async_trait::async_trait;
use scylla::IntoTypedRows;
use crate::config::ImageKind;
use crate::controller::get_bucket_by_id;
use crate::StorageBackend;


pub struct ScyllaBackend {
    table: String,
    connection: session::Session,
}

impl ScyllaBackend {
    pub async fn connect(
        keyspace: String,
        table: Option<String>,
        known_nodes: &[String],
        user: Option<String>,
        password: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut cfg = scylla::SessionConfig::new();
        cfg.add_known_nodes(known_nodes);
        cfg.auth_password = user;
        cfg.auth_password = password;

        let base = scylla::Session::connect(cfg).await?;
        base.use_keyspace(keyspace, false).await?;

        let connection = session::Session::from(base);

        let table = table.unwrap_or_else(|| "lust_image".to_string());
        let qry = format!("CREATE TABLE IF NOT EXISTS {} (\
            bucket_id bigint, \
            sizing_id bigint, \
            image_id uuid, \
            kind text, \
            data blob, \
            PRIMARY KEY ((bucket_id, sizing_id, image_id, kind))
        )", table);
        connection.query(&qry, &[]).await?;

        Ok(Self {
            table,
            connection
        })
    }
}

#[async_trait]
impl StorageBackend for ScyllaBackend {
    async fn store(&self, bucket_id: u32, image_id: Uuid, kind: ImageKind, sizing_id: u32, data: Bytes) -> anyhow::Result<()> {
        let qry = format!("INSERT INTO {table} (bucket_id, sizing_id, image_id, kind, data) VALUES (?, ?, ?, ?, ?);", table = self.table);

        self.connection
            .query_prepared(&qry, (bucket_id as i64, sizing_id as i64,  image_id, kind.as_file_extension(), data.to_vec()))
            .await?;

        Ok(())
    }

    async fn fetch(&self, bucket_id: u32, image_id: Uuid, kind: ImageKind, sizing_id: u32) -> anyhow::Result<Option<Bytes>> {
        let qry = format!("SELECT data FROM {table} WHERE bucket_id = ? AND image_id = ? AND kind = ? AND sizing_id = ?;", table = self.table);

        let buff = self.connection
            .query_prepared(&qry, (bucket_id as i64, image_id, kind.as_file_extension(), sizing_id as i64))
            .await?
            .rows
            .unwrap_or_default()
            .into_typed::<(Vec<u8>,)>()
            .next()
            .transpose()?
            .map(|v| Bytes::from(v.0));

        Ok(buff)
    }

    async fn delete(&self, bucket_id: u32, image_id: Uuid) -> anyhow::Result<Vec<(u32, ImageKind)>> {
        let qry = format!("DELETE FROM {table} WHERE bucket_id = ? AND image_id = ? AND kind = ? AND sizing_id = ?;", table = self.table);

        let bucket = get_bucket_by_id(bucket_id)
            .ok_or_else(|| anyhow!("Bucket does not exist."))?
            .cfg();

        let mut hit_entries = vec![];
        for sizing_id in bucket.sizing_preset_ids().iter().copied() {
            for kind in ImageKind::variants() {
                let values = (bucket_id as i64, image_id, kind.as_file_extension(), sizing_id as i64);
                debug!("Purging image  @ {:?}", &values);

                self.connection
                    .query_prepared(&qry, values)
                    .await?;

                hit_entries.push((sizing_id, *kind))
            }
        }

        Ok(hit_entries)
    }
}

mod session {
    use std::fmt::Debug;
    use scylla::frame::value::ValueList;
    use scylla::query::Query;
    use scylla::transport::errors::{DbError, QueryError};
    use scylla::QueryResult;

    pub struct Session(scylla::CachingSession);

    impl From<scylla::Session> for Session {
        fn from(s: scylla::Session) -> Self {
            Self(scylla::CachingSession::from(s, 100))
        }
    }

    impl AsRef<scylla::Session> for Session {
        fn as_ref(&self) -> &scylla::Session {
            &self.0.session
        }
    }

    impl Session {
        #[instrument(skip(self, query), level = "debug")]
        pub async fn query(
            &self,
            query: &str,
            values: impl ValueList + Debug,
        ) -> Result<QueryResult, QueryError> {
            debug!("executing query {}", query);
            let result = self.0.execute(query, &values).await;

            if let Err(ref e) = result {
                consider_logging_error(e);
            }

            result
        }

        #[instrument(skip(self, query), level = "debug")]
        pub async fn query_prepared(
            &self,
            query: &str,
            values: impl ValueList + Debug,
        ) -> Result<QueryResult, QueryError> {
            debug!("preparing new statement: {}", query);
            let result = self.0.execute(Query::from(query), &values).await;

            match result {
                Ok(res) => Ok(res),
                Err(e) => {
                    consider_logging_error(&e);
                    Err(e)
                },
            }
        }
    }

    fn consider_logging_error(e: &QueryError) {
        if let QueryError::DbError(DbError::AlreadyExists { .. }, ..) = e {
            info!("Keyspace already exists, skipping...");
        }
    }
}
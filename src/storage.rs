use cdrs::authenticators::NoneAuthenticator;
use cdrs::cluster::session::{new as new_session};
use cdrs::cluster::{ClusterTcpConfig, NodeTcpConfigBuilder};
use cdrs::load_balancing::RoundRobin;
use cdrs::query::*;


#[derive(Deserialize)]
pub struct DatabaseConfig {
    clusters: Vec<String>,
    replication_factor: usize,
    class: String,
}


async fn setup() -> anyhow::Result<()> {
    let node = NodeTcpConfigBuilder::new("127.0.0.1:9042", NoneAuthenticator {}).build();
    let cluster_config = ClusterTcpConfig(vec![node]);
    let no_compression =
        new_session(&cluster_config, RoundRobin::new()).await.expect("session should be created");

    let create_ks: &'static str = "CREATE KEYSPACE IF NOT EXISTS test_ks WITH REPLICATION = { \
                                     'class' : 'SimpleStrategy', 'replication_factor' : 1 };";
    no_compression.query(create_ks).await.expect("Keyspace create error");
    Ok(())
}
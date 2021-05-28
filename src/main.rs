mod backends;
mod configure;
mod context;
mod image;
mod response;
mod routes;
mod storage;
mod traits;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate cdrs;

use gotham::middleware::logger::SimpleLogger as GothSimpleLogger;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::{build_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham_derive::{StateData, StaticResponseExtender};

use anyhow::Result;
use clap::{App, Arg, ArgMatches, SubCommand};
use log::LevelFilter;
use serde::Deserialize;
use simple_logger::SimpleLogger;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use crate::configure::StateConfig;
use crate::context::{ImageFormat, ImageGet, ImageRemove};
use crate::storage::{Backend, DatabaseBackend, StorageBackend};
use crate::traits::DatabaseLinker;


static UUID_REGEX: &str = "^[0-9a-fA-F]{8}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{4}\\b-[0-9a-fA-F]{12}$";


#[derive(Deserialize, StateData, StaticResponseExtender)]
struct PathExtractor {
    file_id: Uuid,
}

/// Constructs all the routes for the server.
fn router(backend: storage::StorageBackend, config: StateConfig) -> Result<Router> {
    let base = config.0.base_data_path.clone();
    let pipeline = new_pipeline()
        .add(GothSimpleLogger::new(log::Level::Info))
        .add(StateMiddleware::new(backend))
        .add(StateMiddleware::new(config))
        .build();
    let (chain, pipelines) = single_pipeline(pipeline);

    Ok(build_router(chain, pipelines, |route| {
        route
            .get(&format!(
                "{}/:file_id:{}",
                base,
                UUID_REGEX,
            ))
            .with_path_extractor::<PathExtractor>()
            .with_query_string_extractor::<ImageGet>()
            .to_async(routes::get_file);

        route.post("admin/create").to_async(routes::add_file);

        route
            .delete(&format!("admin/delete/:file_id:{}",UUID_REGEX))
            .with_path_extractor::<ImageRemove>()
            .to_async(routes::remove_file);
    }))
}

/// This will initialise the logger as well as
/// start server and parse args (although not in that order).
#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("lust", LevelFilter::Info)
        .with_module_level("gotham", LevelFilter::Info)
        .init()
        .unwrap();

    let cli_args = parse_args();
    let (name, args) = cli_args.subcommand();
    match name {
        "init" => run_init(args.unwrap()).await,
        "run" => run_server(args.unwrap()).await,
        other => {
            return Err(anyhow::Error::msg(format!(
                "command {} is not supported, only commands (init, run) are supported",
                other,
            )))
        }
    }?;

    Ok(())
}

async fn run_init(args: &ArgMatches<'_>) -> Result<()> {
    let target_backend = args.value_of("backend").expect("backend value not given");

    let example = configure::Config::template(target_backend)?;
    let out = serde_json::to_string_pretty(&example)?;
    fs::write("./config.json", out).await?;

    Ok(())
}

async fn run_server(args: &ArgMatches<'_>) -> Result<()> {
    let cfg = if let Some(cfg) = args.value_of("config") {
        configure::Config::from_file(cfg)
    } else {
        return Err(anyhow::Error::msg(
            "missing required config file, exiting...",
        ));
    }?;

    let backend: storage::StorageBackend = match cfg.database_backend.clone() {
        DatabaseBackend::Cassandra(db_cfg) => {
            let db = backends::cql::Backend::connect(db_cfg).await?;
            let _ = storage::CASSANDRA.set(db);
            StorageBackend::with_backend(Backend::Cassandra)
        }
        DatabaseBackend::Postgres(db_cfg) => {
            let db = backends::sql::PostgresBackend::connect(db_cfg).await?;
            let _ = storage::POSTGRES.set(db);
            StorageBackend::with_backend(Backend::Postgres)
        }
        DatabaseBackend::MySQL(db_cfg) => {
            let db = backends::sql::MySQLBackend::connect(db_cfg).await?;
            let _ = storage::MYSQL.set(db);
            StorageBackend::with_backend(Backend::MySQL)
        }
        DatabaseBackend::Sqlite(db_cfg) => {
            let db = backends::sql::SqliteBackend::connect(db_cfg).await?;
            let _ = storage::SQLITE.set(db);
            StorageBackend::with_backend(Backend::Sqlite)
        }
    };

    let fields: Vec<ImageFormat> = cfg
        .formats
        .iter()
        .filter_map(
            |(format, enabled)| {
                if *enabled {
                    Some(*format)
                } else {
                    None
                }
            },
        )
        .collect();

    let mut presets: Vec<&str> = cfg.size_presets.keys().map(|v| v.as_str()).collect();

    presets.push("original");
    backend.ensure_tables(presets, fields).await?;

    let addr: SocketAddr = format!("{}:{}", &cfg.host, cfg.port).parse()?;
    let state_cfg = StateConfig(Arc::new(cfg));
    let _ = gotham::init_server(addr, router(backend, state_cfg)?).await;

    Ok(())
}

fn parse_args() -> ArgMatches<'static> {
    App::new("Lust")
        .version("0.1.0")
        .author("Harrison Burt <hburt2003@gmail.com>")
        .about("A powerful automatic image server.")
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialises the workspace with a configuration file")
                .version("0.1.0")
                .arg(
                    Arg::with_name("backend")
                        .short("b")
                        .long("backend")
                        .help("The target database backend")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs the server with the given configuration")
                .version("0.1.0")
                .arg(
                    Arg::with_name("config")
                        .short("c")
                        .long("config")
                        .help("The path to a given config file in JSON format.")
                        .takes_value(true)
                        .default_value("config.json"),
                ),
        )
        .get_matches()
}

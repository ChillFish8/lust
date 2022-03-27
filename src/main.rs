mod config;
mod storage;
mod routes;
mod pipelines;
mod controller;
mod utils;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;
use poem::listener::TcpListener;
use poem::{Endpoint, EndpointExt, IntoResponse, Request, Response, Route, Server};
use poem_openapi::OpenApiService;
use tokio::sync::Semaphore;
use tracing::Level;
use crate::controller::BucketController;
use crate::storage::template::StorageBackend;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[macro_use]
extern crate tracing;


#[derive(Debug, Parser)]
pub struct ServerConfig {
    #[clap(short, long, env, default_value = "127.0.0.1:8000")]
    /// The binding address of the server.
    bind: String,

    #[clap(short, long, env)]
    /// The external URL that would be used to access the server if applicable.
    ///
    /// This only affects the documentation.
    docs_url: Option<String>,

    #[clap(long, env, default_value = "info")]
    log_level: Level,

    #[clap(long, env)]
    /// The file path to a given config file.
    ///
    /// This can be either a JSON formatted config or YAML.
    config_file: PathBuf,
}


#[tokio::main]
async fn main() -> Result<()> {
    let args: ServerConfig = ServerConfig::parse();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            format!("{},poem=info,scylla=info,hyper=info", args.log_level),
        );
    }
    tracing_subscriber::fmt::init();

    config::init(&args.config_file).await?;

    let global_limiter = config::config()
        .max_concurrency
        .map(Semaphore::new)
        .map(Arc::new);

    let storage: Arc<dyn StorageBackend> = config::config()
        .backend
        .connect()
        .await?;

    let buckets = config::config()
        .buckets
        .iter()
        .map(|(bucket, cfg)| {
            let bucket_id = crate::utils::crc_hash(bucket);
            let pipeline = cfg.mode.build_pipeline();
            let controller = BucketController::new(
                bucket_id,
                global_limiter.clone(),
                cfg.clone(),
                pipeline,
                storage.clone(),
            );
            (bucket_id, controller)
        })
        .collect();

    controller::init_buckets(buckets);

    let api_service = OpenApiService::new(
        routes::LustApi,
         "Lust API",
        env!("CARGO_PKG_VERSION"),
    )
    .description("The Lust image optimising API.")
    .server(args.docs_url.unwrap_or_else(|| format!("http://{}/v1", args.bind)));

    let ui = api_service.redoc();
    let spec = api_service.spec();

    let app = Route::new()
        .nest("/v1", api_service)
        .nest("/ui", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        .around(log);

    info!("Lust has started!");
    info!(
        "serving requests @ http://{}",
        &args.bind,
    );
    info!("GitHub: https://github.com/chillfish8/lust");
    info!("To ask questions visit: https://github.com/chillfish8/lust/discussions");
    info!(
        "To get started you can check out the documentation @ http://{}/ui",
        &args.bind,
    );

    Server::new(TcpListener::bind(args.bind))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = wait_for_signal().await;
            },
            Some(Duration::from_secs(2)),
        )
        .await?;

    Ok(())
}

async fn wait_for_signal() -> Result<()> {
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }

    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut stream_quit = signal(SignalKind::quit())?;
        let mut stream_interrupt = signal(SignalKind::interrupt())?;
        let mut stream_term = signal(SignalKind::terminate())?;

        tokio::select! {
            _ = stream_quit.recv() => {},
            _ = stream_interrupt.recv() => {},
            _ = stream_term.recv() => {},
        }
    }

    Ok(())
}


async fn log<E: Endpoint>(next: E, req: Request) -> poem::Result<Response> {
    let method = req.method().clone();
    let path = req.uri().clone();

    let start = Instant::now();
    let res = next.call(req).await;
    let elapsed = start.elapsed();

    match res {
        Ok(r) => {
            let resp = r.into_response();

            info!(
                "{} -> {} {} [ {:?} ] - {:?}",
                method.as_str(),
                resp.status().as_u16(),
                resp.status().canonical_reason().unwrap_or(""),
                elapsed,
                path.path(),
            );

            Ok(resp)
        },
        Err(e) => {
            let msg = format!("{}", &e);
            let resp = e.into_response();

            if resp.status().as_u16() >= 500 {
                error!("{}", msg);
            }

            info!(
                "{} -> {} {} [ {:?} ] - {:?}",
                method.as_str(),
                resp.status().as_u16(),
                resp.status().canonical_reason().unwrap_or(""),
                elapsed,
                path.path(),
            );

            Ok(resp)
        },
    }
}
use std::sync::Arc;
use poem::Route;
use poem::http::StatusCode;
use poem_openapi::OpenApiService;
use poem::test::TestClient;
use poem::web::headers;
use tokio::sync::Semaphore;

use crate::{BucketController, config, controller, StorageBackend};

const JIT_CONFIG: &str = include_str!("../tests/configs/jit-mode.yaml");
const AOT_CONFIG: &str = include_str!("../tests/configs/aot-mode.yaml");
const REALTIME_CONFIG: &str = include_str!("../tests/configs/realtime-mode.yaml");
const TEST_IMAGE: &[u8] = include_bytes!("../examples/example.jpeg");

async fn setup_environment(cfg: &str) -> anyhow::Result<TestClient<Route>> {
    config::init_test(cfg)?;

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
            let pipeline = cfg.mode.build_pipeline(cfg);
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

    let app = OpenApiService::new(
        crate::routes::LustApi,
        "Lust API",
        env!("CARGO_PKG_VERSION")
    );

    let app = Route::new().nest("/v1", app);
    Ok(TestClient::new(app))
}


#[tokio::test]
async fn test_basic_aot_upload_retrieval_without_guessing() -> anyhow::Result<()> {
    let app = setup_environment(AOT_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
        .query("format".to_string(), &"jpeg".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    let info = res.json().await;

    let file_id = info
        .value()
        .object()
        .get("image_id")
        .string();

    let res = app.get(format!("/v1/user-profiles/{}", file_id))
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    Ok(())
}

#[tokio::test]
async fn test_basic_aot_upload_retrieval_with_guessing() -> anyhow::Result<()> {
    let app = setup_environment(AOT_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
        .query("format".to_string(), &"jpeg".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    let info = res.json().await;

    let file_id = info
        .value()
        .object()
        .get("image_id")
        .string();

    let res = app.get(format!("/v1/user-profiles/{}", file_id))
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    Ok(())
}

#[tokio::test]
async fn test_basic_jit_upload_retrieval() -> anyhow::Result<()> {
    let app = setup_environment(JIT_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
        .query("format".to_string(), &"jpeg".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    let info = res.json().await;

    let file_id = info
        .value()
        .object()
        .get("image_id")
        .string();

    let res = app.get(format!("/v1/user-profiles/{}", file_id))
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    Ok(())
}

#[tokio::test]
async fn test_basic_realtime_upload_retrieval() -> anyhow::Result<()> {
    let app = setup_environment(REALTIME_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
        .query("format".to_string(), &"jpeg".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    let info = res.json().await;

    let file_id = info
        .value()
        .object()
        .get("image_id")
        .string();

    let res = app.get(format!("/v1/user-profiles/{}", file_id))
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    Ok(())
}

#[tokio::test]
async fn test_realtime_resizing() -> anyhow::Result<()> {
    let app = setup_environment(REALTIME_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
        .query("format".to_string(), &"jpeg".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    let info = res.json().await;

    let file_id = info
        .value()
        .object()
        .get("image_id")
        .string();

    let res = app.get(format!("/v1/user-profiles/{}", file_id))
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    Ok(())
}

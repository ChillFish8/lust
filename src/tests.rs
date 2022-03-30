use std::sync::Arc;
use image::load_from_memory_with_format;
use poem::Route;
use poem::http::StatusCode;
use poem_openapi::OpenApiService;
use poem::test::{TestClient, TestResponse};
use poem::web::headers;
use tokio::sync::Semaphore;

use crate::{BucketController, cache, config, controller, StorageBackend};

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
            let cache = cfg.cache
                .map(cache::new_cache)
                .transpose()?
                .flatten();

            let controller = BucketController::new(
                bucket_id,
                cache,
                global_limiter.clone(),
                cfg.clone(),
                pipeline,
                storage.clone(),
            );
            Ok::<_, anyhow::Error>((bucket_id, controller))
        })
        .collect::<Result<hashbrown::HashMap<_, _>, anyhow::Error>>()?;

    controller::init_buckets(buckets);

    let app = OpenApiService::new(
        crate::routes::LustApi,
        "Lust API",
        env!("CARGO_PKG_VERSION")
    );

    let app = Route::new().nest("/v1", app);
    Ok(TestClient::new(app))
}


async fn validate_image_content(
    res: TestResponse,
    expected_format: image::ImageFormat,
) -> anyhow::Result<()> {
    let body = res.0.into_body().into_bytes().await?;

    load_from_memory_with_format(&body, expected_format)
        .expect("Invalid image returned for expected format");

    Ok(())
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
    res.assert_content_type(&"image/webp".to_string());

    validate_image_content(res, image::ImageFormat::WebP).await?;

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
    res.assert_content_type(&"image/webp".to_string());

    validate_image_content(res, image::ImageFormat::WebP).await?;

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
    res.assert_content_type(&"image/jpeg".to_string());

    validate_image_content(res, image::ImageFormat::Jpeg).await?;

    Ok(())
}

#[tokio::test]
async fn test_jit_upload_custom_format_retrieval() -> anyhow::Result<()> {
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
        .query("format", &"png".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    validate_image_content(res, image::ImageFormat::Png).await?;

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

    validate_image_content(res, image::ImageFormat::Png).await?;

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
        .query("width".to_string(), &"500".to_string())
        .query("height".to_string(), &"500".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_content_type(&"image/png".to_string());

    validate_image_content(res, image::ImageFormat::Png).await?;

    Ok(())
}

#[tokio::test]
async fn test_realtime_resizing_expect_err() -> anyhow::Result<()> {
    let app = setup_environment(REALTIME_CONFIG).await?;

    let res = app.post("/v1/user-profiles")
        .body(TEST_IMAGE)
        .content_type("application/octet-stream".to_string())
        .typed_header(headers::ContentLength(TEST_IMAGE.len() as u64))
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
        .query("width".to_string(), &"500".to_string())
        .send()
        .await;

    res.assert_status(StatusCode::BAD_REQUEST);

    Ok(())
}
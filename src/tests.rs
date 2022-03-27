use poem::{IntoEndpoint, Route};
use poem_openapi::OpenApiService;
use poem::test::TestClient;

use crate::ServerConfig;

const EXAMPLE_CONFIG: &str = include_str!("../examples/example.yaml");
const TEST_IMAGE: &[u8] = include_bytes!()

fn setup_environment() -> TestClient<Route> {
    let app = OpenApiService::new(
        crate::routes::LustApi,
        "Lust API",
        env!("CARGO_PKG_VERSION")
    );

    let app = Route::new().nest("/v1", app);
    TestClient::new(app)
}


#[tokio::test]
async fn test_basic_aot_upload_retrieval() -> anyhow::Result<()> {
    crate::config::init_test(EXAMPLE_CONFIG)?;
    let app = setup_environment();

    app.post("/v1/user-profiles")
        .body()

    Ok(())
}

#[tokio::test]
async fn test_basic_jit_upload_retrieval() -> anyhow::Result<()> {
    crate::config::init_test(EXAMPLE_CONFIG)?;
    let app = setup_environment();

    Ok(())
}

#[tokio::test]
async fn test_basic_realtime_upload_retrieval() -> anyhow::Result<()> {
    crate::config::init_test(EXAMPLE_CONFIG)?;
    let app = setup_environment();

    Ok(())
}

#[tokio::test]
async fn test_realtime_resizing() -> anyhow::Result<()> {
    crate::config::init_test(EXAMPLE_CONFIG)?;
    let app = setup_environment();

    Ok(())
}

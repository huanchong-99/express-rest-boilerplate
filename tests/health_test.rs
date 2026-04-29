//! Integration tests for the health check endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// Build a minimal test router that directly uses the handler from the library crate.
fn create_health_test_app() -> Router {
    Router::new().route("/v1/health-check", get(express_rest_boilerplate::handlers::health::health_check))
}

#[tokio::test]
async fn health_check_returns_200_ok() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_health_test_app();
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check").body(Body::empty())?
    ).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await?.to_bytes();
    let text = String::from_utf8(body.to_vec())?;
    assert_eq!(text, "\"OK\"");
    Ok(())
}

#[tokio::test]
async fn health_check_returns_json_content_type() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_health_test_app();
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check").body(Body::empty())?
    ).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let ct = response.headers().get("content-type").ok_or("missing content-type")?;
    assert!(ct.to_str()?.contains("application/json"));
    Ok(())
}

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
async fn health_check_returns_200_ok() {
    let app = create_health_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/health-check")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("should succeed")
        .to_bytes();
    let text = String::from_utf8(body.to_vec()).expect("should succeed");
    assert_eq!(text, "\"OK\"");
}

#[tokio::test]
async fn health_check_returns_json_content_type() {
    let app = create_health_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/health-check")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .expect("content-type header should be present");
    assert!(content_type.to_str().expect("should succeed").contains("application/json"));
}

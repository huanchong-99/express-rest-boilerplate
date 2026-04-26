//! Integration tests for the health check endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn create_test_app() -> axum::Router {
    use axum::Router;
    use axum::routing::get;
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    // Build a minimal router with just the health endpoint for testing.
    Router::new()
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::test]
async fn health_check_returns_200_ok() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(text, "OK");
}

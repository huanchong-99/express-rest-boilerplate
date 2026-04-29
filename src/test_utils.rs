//! Test infrastructure — helpers for creating test app state and mock requests.
//!
//! These utilities support integration tests in `tests/` and
//! `#[cfg(test)]` modules within the crate. They DO NOT connect to a
//! real database; they provide helpers for building test routers and
//! making mock HTTP requests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::app_state::AppState;
use crate::config::AppConfig;
use crate::create_app;

/// Build a test `AppConfig` with safe defaults.
///
/// These values are intentionally non-secret and only used for tests.
pub fn test_config() -> AppConfig {
    AppConfig {
        database_url: "postgres://postgres:postgres@localhost:5432/express_rest_boilerplate_test"
            .into(),
        token_signing_key: "test-only-secret-not-for-production".into(),
        jwt_expiration_minutes: 15,
        port: 0, // unused in tests
        host: "127.0.0.1".into(),
        env: "test".into(),
    }
}

/// Build a test Axum app with the given state.
///
/// If you need a DB-backed app, provide an `AppState` with a real `PgPool`.
/// For handler-only tests that don't touch the DB, use `create_test_app_without_db()`.
pub fn create_test_app(state: AppState) -> Router {
    create_app(state)
}

/// Build a minimal test router without a database connection.
///
/// Use this for testing handlers that don't require database access
/// (e.g., health check, validation-only endpoints).
pub fn create_test_app_without_db() -> Router {
    // Build a simple router with just the health check handler
    Router::new().route(
        "/v1/health-check",
        axum::routing::get(crate::handlers::health::health_check),
    )
}

/// Helper to send a request to a test app and return the response.
pub async fn send_request(app: Router, request: Request<Body>) -> axum::http::Response<Body> {
    app.oneshot(request).await.expect("request should succeed")
}

/// Helper to extract the response body as a string.
pub async fn body_to_string(body: Body) -> String {
    let bytes = body
        .collect()
        .await
        .expect("body should be collectable")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body should be valid UTF-8")
}

/// Helper to assert a response status code.
pub fn assert_status(response: &axum::http::Response<Body>, expected: StatusCode) {
    assert_eq!(response.status(), expected);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_via_test_app() {
        let app = create_test_app_without_db();
        let request = Request::builder()
            .uri("/v1/health-check")
            .body(Body::empty())
            .unwrap();
        let response = send_request(app, request).await;
        assert_status(&response, StatusCode::OK);
        let body = body_to_string(response.into_body()).await;
        assert_eq!(body, "\"OK\"");
    }
}

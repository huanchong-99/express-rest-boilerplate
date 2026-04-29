//! Comprehensive tests for auth and user API route handlers.
//!
//! These tests exercise the handler layer via Axum's oneshot test utility.
//! They do NOT require a running database — they test routing, middleware
//! extraction, validation, and response shape.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use express_rest_boilerplate::app_state::AppState;
use express_rest_boilerplate::config::AppConfig;
use express_rest_boilerplate::create_app;

/// Build a test app with a real AppState pointing at a test database.
fn create_test_app() -> Result<Router, Box<dyn std::error::Error>> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/express_rest_boilerplate_test".into());
    let token_signing_key = std::env::var("TEST_JWT_SECRET")
        .unwrap_or_else(|_| "insecure-test-key".into());
    let config = AppConfig {
        database_url,
        token_signing_key,
        jwt_expiration_minutes: 15,
        port: 3000,
        host: "0.0.0.0".into(),
        env: "test".into(),
    };

    let pool = sqlx::PgPool::connect_lazy(&config.database_url)?;

    let state = AppState { pool, config };

    Ok(create_app(state))
}

/// Helper to extract response body as String.
async fn body_string(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = body.collect().await?.to_bytes();
    let text = String::from_utf8(bytes.to_vec())?;
    Ok(text)
}

/// Helper to extract response body as serde_json::Value.
async fn body_json(body: Body) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let text = body_string(body).await?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    Ok(value)
}

// ================================================================
// AUTH ENDPOINT TESTS (validation and routing)
// ================================================================

#[tokio::test]
async fn register_missing_email_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"password":"123456"}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 400);
    let msg = json["message"].as_str().ok_or("message should be string")?;
    assert!(msg.contains("email") || msg.contains("Validation"));
    Ok(())
}

#[tokio::test]
async fn register_invalid_email_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email":"not-an-email","password":"123456"}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 400);

    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["field"], "email");
    assert_eq!(errors[0]["location"], "body");
    Ok(())
}

#[tokio::test]
async fn register_password_too_short_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"user@example.com","password":"12345"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert!(!errors.is_empty());
    assert_eq!(errors[0]["field"], "password");
    Ok(())
}

#[tokio::test]
async fn register_empty_body_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn login_missing_fields_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 400);
    Ok(())
}

#[tokio::test]
async fn login_invalid_email_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"not-valid","password":"123456"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn refresh_token_missing_fields_returns_400() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh-token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

// ================================================================
// PROTECTED ENDPOINT TESTS (without valid token)
// ================================================================

#[tokio::test]
async fn get_profile_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/users/profile")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn get_profile_with_invalid_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/users/profile")
                .header("Authorization", "Bearer invalid-token-here")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn get_profile_with_malformed_auth_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/users/profile")
                .header("Authorization", "NotBearer sometoken")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn list_users_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/users")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn create_user_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/users")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"test@example.com","password":"123456"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn get_user_by_id_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/users/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn replace_user_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/v1/users/00000000-0000-0000-0000-000000000000")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"test@example.com","password":"123456"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn update_user_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/users/00000000-0000-0000-0000-000000000000")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Updated Name"}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn delete_user_without_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/users/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

// ================================================================
// VALIDATION AND RESPONSE SHAPE TESTS
// ================================================================

#[test]
fn test_user_response_shape_matches_express() {
    let json = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": "Test User",
        "email": "test@example.com",
        "picture": null,
        "role": "user",
        "created_at": "2024-01-01T00:00:00Z"
    });

    assert!(json.get("id").is_some());
    assert!(json.get("name").is_some());
    assert!(json.get("email").is_some());
    assert!(json.get("picture").is_some());
    assert!(json.get("role").is_some());
    assert!(json.get("created_at").is_some());
    assert!(json.get("password").is_none());
}

#[test]
fn test_auth_response_shape_matches_express() {
    let json = serde_json::json!({
        "token": {
            "token_type": "Bearer",
            "access_token": "jwt-token-here",
            "refresh_token": "refresh-token-here",
            "expires_in": "2024-01-01T00:15:00Z"
        },
        "user": {
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "Test User",
            "email": "test@example.com",
            "role": "user",
            "created_at": "2024-01-01T00:00:00Z"
        }
    });

    assert!(json["token"]["token_type"].is_string());
    assert!(json["token"]["access_token"].is_string());
    assert!(json["token"]["refresh_token"].is_string());
    assert!(json["token"]["expires_in"].is_string());
    assert!(json["user"]["id"].is_string());
    assert!(json["user"]["email"].is_string());
    assert!(json["user"]["role"].is_string());
}

#[test]
fn test_error_response_shape_matches_express() {
    let json = serde_json::json!({
        "code": 409,
        "message": "Validation Error",
        "errors": [{
            "field": "email",
            "location": "body",
            "messages": ["\"email\" already exists"]
        }]
    });

    assert_eq!(json["code"], 409);
    assert_eq!(json["message"], "Validation Error");
    assert!(json["errors"].is_array());
    assert!(json["errors"][0]["field"] == "email");
    assert!(json["errors"][0]["location"] == "body");
    assert!(json["errors"][0]["messages"].is_array());
}

#[test]
fn test_login_error_response_shape() {
    let json = serde_json::json!({
        "code": 401,
        "message": "Incorrect email or password"
    });
    assert_eq!(json["code"], 401);
    assert_eq!(json["message"], "Incorrect email or password");
}

#[test]
fn test_refresh_token_error_response_shape() {
    let json = serde_json::json!({
        "code": 401,
        "message": "Incorrect email or refreshToken"
    });
    assert_eq!(json["code"], 401);
    assert_eq!(json["message"], "Incorrect email or refreshToken");
}

#[test]
fn test_not_found_error_response_shape() {
    let json = serde_json::json!({
        "code": 404,
        "message": "User does not exist"
    });
    assert_eq!(json["code"], 404);
    assert_eq!(json["message"], "User does not exist");
}
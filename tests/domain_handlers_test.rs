//! Comprehensive unit and integration tests for domain API handlers.
//!
//! Tests cover:
//! - Auth endpoint validation (register, login, refresh)
//! - User CRUD validation
//! - Response shape serialization
//! - Model validation
//! - Error response format
//! - Authorization logic
//! - Token creation/decoding
//! - Route coverage

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use express_rest_boilerplate::app_state::AppState;
use express_rest_boilerplate::config::AppConfig;
use express_rest_boilerplate::create_app;

fn create_test_app() -> Router {
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
    let pool = sqlx::PgPool::connect_lazy(&config.database_url).expect("Failed to create pool");
    let state = AppState { pool, config };
    create_app(state)
}

async fn body_string(body: Body) -> String {
    let bytes = body.collect().await.expect("collectable").to_bytes();
    String::from_utf8(bytes.to_vec()).expect("valid UTF-8")
}

async fn body_json(body: Body) -> serde_json::Value {
    let text = body_string(body).await;
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("Failed to parse JSON: {e}"))
}

// ================================================================
// AUTH VALIDATION TESTS
// ================================================================

#[tokio::test]
async fn extra_register_password_too_long_returns_400() {
    let app = create_test_app();
    let long_password = "x".repeat(129);
    let body = serde_json::json!({"email":"user@example.com","password":long_password}).to_string();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(body)).expect("valid"),
    ).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await;
    let errors = json["errors"].as_array().expect("errors");
    assert_eq!(errors[0]["field"], "password");
}

#[tokio::test]
async fn extra_register_valid_payload_shape_accepted() {
    let app = create_test_app();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"user@example.com","password":"123456"}"#)).expect("valid"),
    ).await.expect("oneshot");
    assert_ne!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn extra_login_password_exceeds_max_length_returns_400() {
    let app = create_test_app();
    let long_password = "x".repeat(129);
    let body = serde_json::json!({"email":"user@example.com","password":long_password}).to_string();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(body)).expect("valid"),
    ).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn extra_refresh_token_invalid_email_returns_400() {
    let app = create_test_app();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/refresh-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"bad-email","refreshToken":"some-token"}"#)).expect("valid"),
    ).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ================================================================
// ROUTE COVERAGE
// ================================================================

#[tokio::test]
async fn extra_health_check_via_v1_route() {
    let app = create_test_app();
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check").body(Body::empty()).expect("valid"),
    ).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response.into_body()).await;
    assert_eq!(body, "\"OK\"");
}

#[tokio::test]
async fn extra_non_existent_route_returns_404() {
    let app = create_test_app();
    let response = app.oneshot(
        Request::builder().uri("/v1/nonexistent").body(Body::empty()).expect("valid"),
    ).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn extra_post_register_without_content_type_4xx() {
    let app = create_test_app();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .body(Body::from(r#"{"email":"user@example.com","password":"123456"}"#)).expect("valid"),
    ).await.expect("oneshot");
    assert!(response.status().is_client_error());
}

// ================================================================
// SERIALIZATION TESTS
// ================================================================

#[test]
fn extra_user_response_serialization() {
    use chrono::{TimeZone, Utc};
    use express_rest_boilerplate::models::user::UserResponse;
    use uuid::Uuid;
    let user = UserResponse {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        name: Some("Test User".into()),
        email: "test@example.com".into(),
        picture: None,
        role: "user".into(),
        created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    };
    let json = serde_json::to_value(&user).unwrap();
    assert_eq!(json["id"], "00000000-0000-0000-0000-000000000001");
    assert_eq!(json["name"], "Test User");
    assert!(json["picture"].is_null());
    assert!(json.get("password").is_none());
}

#[test]
fn extra_user_response_null_name() {
    use chrono::{TimeZone, Utc};
    use express_rest_boilerplate::models::user::UserResponse;
    use uuid::Uuid;
    let user = UserResponse {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
        name: None, email: "noname@example.com".into(), picture: None,
        role: "admin".into(), created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    };
    let json = serde_json::to_value(&user).unwrap();
    assert!(json["name"].is_null());
    assert_eq!(json["role"], "admin");
}

#[test]
fn extra_field_error_serialization() {
    use express_rest_boilerplate::errors::FieldError;
    let fe = FieldError::new("email", "body", vec!["duplicate".into()]);
    let json = serde_json::to_value(&fe).unwrap();
    assert_eq!(json["field"], "email");
    assert_eq!(json["location"], "body");
}

// ================================================================
// MODEL VALIDATION
// ================================================================

#[test]
fn extra_new_user_valid() {
    use express_rest_boilerplate::models::user::NewUser; use validator::Validate;
    let u = NewUser { email: "user@example.com".into(), password: "password123".into(), name: Some("Test".into()), role: Some("admin".into()) };
    assert!(u.validate().is_ok());
}
#[test]
fn extra_new_user_bad_email() {
    use express_rest_boilerplate::models::user::NewUser; use validator::Validate;
    let u = NewUser { email: "bad".into(), password: "password123".into(), name: None, role: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_new_user_short_pw() {
    use express_rest_boilerplate::models::user::NewUser; use validator::Validate;
    let u = NewUser { email: "user@example.com".into(), password: "12345".into(), name: None, role: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_new_user_long_pw() {
    use express_rest_boilerplate::models::user::NewUser; use validator::Validate;
    let u = NewUser { email: "user@example.com".into(), password: "x".repeat(129), name: None, role: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_new_user_long_name() {
    use express_rest_boilerplate::models::user::NewUser; use validator::Validate;
    let u = NewUser { email: "user@example.com".into(), password: "password123".into(), name: Some("x".repeat(129)), role: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_update_user_valid() {
    use express_rest_boilerplate::models::user::UpdateUser; use validator::Validate;
    let u = UpdateUser { email: Some("new@example.com".into()), password: None, name: Some("New".into()), role: None, picture: None };
    assert!(u.validate().is_ok());
}
#[test]
fn extra_update_user_bad_email() {
    use express_rest_boilerplate::models::user::UpdateUser; use validator::Validate;
    let u = UpdateUser { email: Some("bad".into()), password: None, name: None, role: None, picture: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_update_user_short_pw() {
    use express_rest_boilerplate::models::user::UpdateUser; use validator::Validate;
    let u = UpdateUser { email: None, password: Some("12345".into()), name: None, role: None, picture: None };
    assert!(u.validate().is_err());
}
#[test]
fn extra_update_user_all_none() {
    use express_rest_boilerplate::models::user::UpdateUser; use validator::Validate;
    let u = UpdateUser { email: None, password: None, name: None, role: None, picture: None };
    assert!(u.validate().is_ok());
}

// ================================================================
// APP ERROR TESTS
// ================================================================

#[test]
fn extra_sqlx_row_not_found_maps_to_user_not_found() {
    use express_rest_boilerplate::errors::AppError;
    let app_err: AppError = sqlx::Error::RowNotFound.into();
    match app_err { AppError::UserNotFound => {} other => panic!("Expected UserNotFound, got {:?}", other) }
}
#[test]
fn extra_error_response_unauthorized() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::Unauthorized.into_response().status(), StatusCode::UNAUTHORIZED);
}
#[test]
fn extra_error_response_forbidden() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::Forbidden.into_response().status(), StatusCode::FORBIDDEN);
}
#[test]
fn extra_error_response_not_found() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::UserNotFound.into_response().status(), StatusCode::NOT_FOUND);
}
#[test]
fn extra_error_response_duplicate_email() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::DuplicateEmail.into_response().status(), StatusCode::CONFLICT);
}
#[test]
fn extra_error_response_bad_request() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::BadRequest("test".into()).into_response().status(), StatusCode::BAD_REQUEST);
}
#[test]
fn extra_error_response_internal() {
    use axum::http::StatusCode; use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;
    assert_eq!(AppError::Internal("err".into()).into_response().status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// ================================================================
// AUTHORIZATION TESTS
// ================================================================

fn make_test_user(id: &str, role: &str) -> express_rest_boilerplate::models::user::User {
    use chrono::{TimeZone, Utc};
    express_rest_boilerplate::models::user::User {
        id: uuid::Uuid::parse_str(id).unwrap(), email: "test@example.com".into(),
        password: "hash".into(), name: Some("Test".into()), role: role.into(),
        picture: None, facebook_id: None, google_id: None,
        created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    }
}

#[test]
fn extra_authorize_admin_any_user() {
    use express_rest_boilerplate::middleware::auth::authorize_user_access; use uuid::Uuid;
    let admin = make_test_user("00000000-0000-0000-0000-000000000001", "admin");
    let target = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    assert!(authorize_user_access(&admin, target).is_ok());
}
#[test]
fn extra_authorize_user_self() {
    use express_rest_boilerplate::middleware::auth::authorize_user_access;
    let user = make_test_user("00000000-0000-0000-0000-000000000001", "user");
    assert!(authorize_user_access(&user, user.id).is_ok());
}
#[test]
fn extra_authorize_user_other_forbidden() {
    use express_rest_boilerplate::middleware::auth::authorize_user_access; use uuid::Uuid;
    let user = make_test_user("00000000-0000-0000-0000-000000000001", "user");
    let other = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    assert!(authorize_user_access(&user, other).is_err());
}

// ================================================================
// TOKEN TESTS
// ================================================================

#[test]
fn extra_token_round_trip() {
    use express_rest_boilerplate::middleware::auth::{create_access_token, decode_access_token}; use uuid::Uuid;
    let user_id = Uuid::new_v4();
    let (token, _) = create_access_token(user_id, "test-key", 15).unwrap();
    let claims = decode_access_token(&token, "test-key").unwrap();
    assert_eq!(claims.sub, user_id.to_string());
}
#[test]
fn extra_token_wrong_key() {
    use express_rest_boilerplate::middleware::auth::{create_access_token, decode_access_token}; use uuid::Uuid;
    let user_id = Uuid::new_v4();
    let (token, _) = create_access_token(user_id, "key-a", 15).unwrap();
    assert!(decode_access_token(&token, "key-b").is_err());
}
#[test]
fn extra_user_to_response() {
    use express_rest_boilerplate::models::user::UserResponse;
    let user = make_test_user("00000000-0000-0000-0000-000000000001", "user");
    let response: UserResponse = user.into();
    assert_eq!(response.email, "test@example.com");
    assert_eq!(response.role, "user");
}
#[test]
fn extra_is_valid_role() {
    use express_rest_boilerplate::middleware::auth::is_valid_role;
    assert!(is_valid_role("user"));
    assert!(is_valid_role("admin"));
    assert!(!is_valid_role("superadmin"));
    assert!(!is_valid_role(""));
}
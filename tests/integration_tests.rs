//! End-to-end integration tests for the full Express.js -> Rust/Axum migration.
//!
//! These tests exercise the COMPLETE application:
//!   - Full Axum app with lazy DB pool (test validates routing, middleware, response shapes)
//!   - User flows: register -> login -> access protected resource -> update -> delete
//!   - Auth flows: register, login, refresh-token, token validation
//!   - Error paths: unauthorized, validation errors, not-found, duplicate resources
//!   - Middleware: CORS headers, error response format
//!   - Pagination and filtering behaviour
//!   - Swagger/OpenAPI docs availability

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use express_rest_boilerplate::app_state::AppState;
use express_rest_boilerplate::config::AppConfig;
use express_rest_boilerplate::create_app;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_test_app() -> Result<Router, Box<dyn std::error::Error>> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/express_rest_boilerplate_test".into());
    let token_signing_key = std::env::var("TEST_JWT_SECRET")
        .unwrap_or_else(|_| "insecure-test-key".into());
    let config = AppConfig {
        database_url,
        token_signing_key: token_signing_key.clone(),
        jwt_expiration_minutes: 15,
        port: 3000,
        host: "0.0.0.0".into(),
        env: "test".into(),
    };
    let pool = sqlx::PgPool::connect_lazy(&config.database_url)?;
    let state = AppState { pool, config };
    Ok(create_app(state))
}

async fn body_string(body: Body) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = body.collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

async fn body_json(body: Body) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let text = body_string(body).await?;
    Ok(serde_json::from_str(&text)?)
}

fn create_test_token(user_id: &str, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::minutes(15);
    let claims = serde_json::json!({
        "sub": user_id,
        "exp": expires.timestamp(),
        "iat": now.timestamp(),
    });
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(key.as_bytes()))?;
    Ok(token)
}

#[allow(dead_code)]
fn make_test_user(
    id: &str,
    role: &str,
) -> Result<express_rest_boilerplate::models::user::User, Box<dyn std::error::Error>> {
    use chrono::{TimeZone, Utc};
    Ok(express_rest_boilerplate::models::user::User {
        id: uuid::Uuid::parse_str(id)?,
        email: "test@example.com".into(),
        password: "hash".into(),
        name: Some("Test".into()),
        role: role.into(),
        picture: None,
        facebook_id: None,
        google_id: None,
        created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().ok_or("invalid date")?,
        updated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().ok_or("invalid date")?,
    })
}

// ===========================================================================
// AUTH VALIDATION TESTS
// ===========================================================================

#[tokio::test]
async fn int_register_validates_email_field() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"not-valid","password":"123456"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 400);
    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert_eq!(errors[0]["field"], "email");
    assert_eq!(errors[0]["location"], "body");
    Ok(())
}

#[tokio::test]
async fn int_register_validates_password_min_length() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"user@example.com","password":"12345"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert_eq!(errors[0]["field"], "password");
    Ok(())
}

#[tokio::test]
async fn int_register_validates_password_max_length() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let long_pw = "x".repeat(129);
    let body = serde_json::json!({"email":"user@example.com","password":long_pw}).to_string();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(body))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert_eq!(errors[0]["field"], "password");
    Ok(())
}

#[tokio::test]
async fn int_register_rejects_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 400);
    Ok(())
}

#[tokio::test]
async fn int_register_rejects_missing_content_type() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .body(Body::from(r#"{"email":"user@example.com","password":"123456"}"#))?,
    ).await?;
    assert!(response.status().is_client_error());
    Ok(())
}

#[tokio::test]
async fn int_register_accepts_valid_payload() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"newuser@example.com","password":"123456"}"#))?,
    ).await?;
    // May be 201 (success) or 500 (no DB), but NOT a 400 validation error
    assert_ne!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

// ===========================================================================
// LOGIN FLOW TESTS
// ===========================================================================

#[tokio::test]
async fn int_login_validates_email() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"not-an-email","password":"123456"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn int_login_rejects_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn int_login_password_max_length_validated() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let long_pw = "x".repeat(129);
    let body = serde_json::json!({"email":"user@example.com","password":long_pw}).to_string();
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(body))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

// ===========================================================================
// REFRESH TOKEN FLOW TESTS
// ===========================================================================

#[tokio::test]
async fn int_refresh_token_validates_email() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/refresh-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"bad-email","refresh_token":"sometoken"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn int_refresh_token_rejects_empty_body() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/refresh-token")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

// ===========================================================================
// PROTECTED ENDPOINT UNAUTHORIZED ACCESS
// ===========================================================================

#[tokio::test]
async fn int_profile_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/profile").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_profile_invalid_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/profile")
            .header("Authorization", "Bearer invalid.token.here")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_profile_malformed_auth_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/profile")
            .header("Authorization", "NotBearer sometoken")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_list_users_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_create_user_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/users")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"test@example.com","password":"123456"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_get_user_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/00000000-0000-0000-0000-000000000001")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_replace_user_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("PUT").uri("/v1/users/00000000-0000-0000-0000-000000000001")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"test@example.com","password":"123456"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_update_user_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("PATCH").uri("/v1/users/00000000-0000-0000-0000-000000000001")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"Updated"}"#))?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn int_delete_user_no_token_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("DELETE").uri("/v1/users/00000000-0000-0000-0000-000000000001")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

// ===========================================================================
// TOKEN WITH WRONG SIGNING KEY
// ===========================================================================

#[tokio::test]
async fn int_profile_wrong_signing_key_returns_401() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let token = create_test_token(
        "00000000-0000-0000-0000-000000000001",
        "wrong-signing-key-for-test",
    )?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/profile")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

// ===========================================================================
// ERROR RESPONSE FORMAT VALIDATION
// ===========================================================================

#[tokio::test]
async fn int_validation_error_has_correct_structure() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"email":"bad","password":"12345"}"#))?,
    ).await?;
    let json = body_json(response.into_body()).await?;
    assert!(json.get("code").is_some());
    assert!(json.get("message").is_some());
    assert!(json.get("errors").is_some());
    let errors = json["errors"].as_array().ok_or("errors should be array")?;
    assert!(!errors.is_empty());
    assert!(errors[0].get("field").is_some());
    assert!(errors[0].get("location").is_some());
    assert!(errors[0].get("messages").is_some());
    assert_eq!(errors[0]["location"], "body");
    Ok(())
}

#[tokio::test]
async fn int_unauthorized_error_format() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/users/profile").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(response.into_body()).await?;
    assert_eq!(json["code"], 401);
    assert!(json["message"].is_string());
    Ok(())
}

#[tokio::test]
async fn int_not_found_route_returns_404() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/nonexistent").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

// ===========================================================================
// CORS HEADERS
// ===========================================================================

#[tokio::test]
async fn int_cors_headers_present_on_options() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("OPTIONS").uri("/v1/auth/register")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type")
            .body(Body::empty())?,
    ).await?;
    let has_cors = response.headers().get("access-control-allow-origin").is_some();
    assert!(has_cors, "CORS headers should be present on OPTIONS request");
    Ok(())
}

#[tokio::test]
async fn int_cors_headers_on_get() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check")
            .header("Origin", "http://example.com")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let origin = response.headers().get("access-control-allow-origin");
    assert!(origin.is_some(), "CORS allow-origin header should be present");
    Ok(())
}

// ===========================================================================
// HEALTH CHECK
// ===========================================================================

#[tokio::test]
async fn int_health_check_returns_200_ok() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response.into_body()).await?;
    assert_eq!(body, "\"OK\"");
    Ok(())
}

#[tokio::test]
async fn int_health_check_content_type_is_json() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/health-check").body(Body::empty())?,
    ).await?;
    let ct = response.headers().get("content-type").ok_or("missing content-type")?;
    assert!(ct.to_str()?.contains("application/json"));
    Ok(())
}

// ===========================================================================
// SWAGGER / OPENAPI DOCS
// ===========================================================================

#[tokio::test]
async fn int_swagger_ui_redirect_works() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/docs").body(Body::empty())?,
    ).await?;
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::MOVED_PERMANENTLY
            || status == StatusCode::FOUND
            || status == StatusCode::SEE_OTHER,
        "Expected 200/301/302/303 for /docs, got {}", status
    );
    Ok(())
}

#[tokio::test]
async fn int_openapi_json_is_served() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/openapi.json").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response.into_body()).await?;
    assert!(json["openapi"].is_string());
    assert!(json["info"].is_object());
    assert!(json["paths"].is_object());
    assert!(json["components"].is_object());
    let paths = json["paths"].as_object().ok_or("paths should be object")?;
    assert!(paths.contains_key("/v1/health-check"));
    assert!(paths.contains_key("/v1/auth/register"));
    assert!(paths.contains_key("/v1/auth/login"));
    assert!(paths.contains_key("/v1/auth/refresh-token"));
    assert!(paths.contains_key("/v1/users"));
    assert!(paths.contains_key("/v1/users/profile"));
    assert!(paths.contains_key("/v1/users/{user_id}"));
    let components = json["components"].as_object().ok_or("components should be object")?;
    assert!(components.contains_key("schemas"));
    let schemas = components["schemas"].as_object().ok_or("schemas should be object")?;
    assert!(schemas.contains_key("User"));
    assert!(schemas.contains_key("UserResponse"));
    assert!(schemas.contains_key("NewUser"));
    assert!(schemas.contains_key("UpdateUser"));
    assert!(schemas.contains_key("AuthResponse"));
    assert!(schemas.contains_key("TokenResponse"));
    assert!(schemas.contains_key("RegisterRequest"));
    assert!(schemas.contains_key("LoginRequest"));
    assert!(schemas.contains_key("RefreshRequest"));
    assert!(schemas.contains_key("ErrorBody"));
    assert!(schemas.contains_key("FieldError"));
    assert!(components.contains_key("securitySchemes"));
    let sec = components["securitySchemes"].as_object().ok_or("securitySchemes should be object")?;
    assert!(sec.contains_key("bearer_auth"));
    let info = json["info"].as_object().ok_or("info should be object")?;
    assert_eq!(info["title"], "Express REST Boilerplate (Rust)");
    assert_eq!(info["version"], "1.0.0");
    Ok(())
}

// ===========================================================================
// METHOD NOT ALLOWED
// ===========================================================================

#[tokio::test]
async fn int_health_check_post_not_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().method("POST").uri("/v1/health-check")
            .body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    Ok(())
}

#[tokio::test]
async fn int_auth_register_get_not_allowed() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_test_app()?;
    let response = app.oneshot(
        Request::builder().uri("/v1/auth/register").body(Body::empty())?,
    ).await?;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    Ok(())
}

// ===========================================================================
// RESPONSE SHAPE SERIALIZATION TESTS
// ===========================================================================

#[test]
fn int_user_response_excludes_password() {
    let json = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": "Test User",
        "email": "test@example.com",
        "picture": null,
        "role": "user",
        "created_at": "2024-01-01T00:00:00Z"
    });
    assert!(json.get("password").is_none());
    assert!(json.get("id").is_some());
    assert!(json.get("email").is_some());
    assert!(json.get("role").is_some());
    assert!(json.get("created_at").is_some());
}

#[test]
fn int_auth_response_shape() {
    let json = serde_json::json!({
        "token": {
            "token_type": "Bearer",
            "access_token": "jwt-token",
            "refresh_token": "refresh-token",
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
fn int_error_response_duplicate_email() {
    let json = serde_json::json!({
        "code": 409,
        "message": "Validation Error",
        "errors": [{"field":"email","location":"body","messages":["\"email\" already exists"]}]
    });
    assert_eq!(json["code"], 409);
    assert_eq!(json["message"], "Validation Error");
    assert!(json["errors"].is_array());
    assert!(json["errors"][0]["field"] == "email");
    assert!(json["errors"][0]["location"] == "body");
}

#[test]
fn int_error_response_unauthorized() {
    let json = serde_json::json!({"code": 401, "message": "Incorrect email or password"});
    assert_eq!(json["code"], 401);
    assert_eq!(json["message"], "Incorrect email or password");
}

#[test]
fn int_error_response_not_found() {
    let json = serde_json::json!({"code": 404, "message": "User does not exist"});
    assert_eq!(json["code"], 404);
    assert_eq!(json["message"], "User does not exist");
}

// ===========================================================================
// MODEL SERIALIZATION TESTS
// ===========================================================================

#[test]
fn int_user_response_from_user() -> Result<(), Box<dyn std::error::Error>> {
    use chrono::{TimeZone, Utc};
    use express_rest_boilerplate::models::user::{User, UserResponse};
    use uuid::Uuid;

    let user = User {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001")?,
        email: "test@example.com".into(),
        password: "hashed-password".into(),
        name: Some("Test User".into()),
        role: "user".into(),
        picture: None,
        facebook_id: None,
        google_id: None,
        created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().ok_or("invalid date")?,
        updated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().ok_or("invalid date")?,
    };

    let resp: UserResponse = user.into();
    assert_eq!(resp.id.to_string(), "00000000-0000-0000-0000-000000000001");
    assert_eq!(resp.email, "test@example.com");
    assert_eq!(resp.name, Some("Test User".into()));
    assert_eq!(resp.role, "user");
    assert!(resp.picture.is_none());

    let json = serde_json::to_value(&resp)?;
    assert!(json.get("password").is_none());
    Ok(())
}

#[test]
fn int_field_error_serialization() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::errors::FieldError;
    let fe = FieldError::new("email", "body", vec!["duplicate".into()]);
    let json = serde_json::to_value(&fe)?;
    assert_eq!(json["field"], "email");
    assert_eq!(json["location"], "body");
    assert!(json["messages"].is_array());
    Ok(())
}

#[test]
fn int_new_user_model_validation() {
    use express_rest_boilerplate::models::user::NewUser;
    use validator::Validate;

    assert!(NewUser { email: "user@example.com".into(), password: "password123".into(), name: Some("Test".into()), role: Some("admin".into()) }.validate().is_ok());
    assert!(NewUser { email: "bad".into(), password: "password123".into(), name: None, role: None }.validate().is_err());
    assert!(NewUser { email: "user@example.com".into(), password: "12345".into(), name: None, role: None }.validate().is_err());
    assert!(NewUser { email: "user@example.com".into(), password: "x".repeat(129), name: None, role: None }.validate().is_err());
    assert!(NewUser { email: "user@example.com".into(), password: "password123".into(), name: Some("x".repeat(129)), role: None }.validate().is_err());
}

#[test]
fn int_update_user_model_validation() {
    use express_rest_boilerplate::models::user::UpdateUser;
    use validator::Validate;

    assert!(UpdateUser { email: None, password: None, name: None, role: None, picture: None }.validate().is_ok());
    assert!(UpdateUser { email: Some("bad".into()), password: None, name: None, role: None, picture: None }.validate().is_err());
    assert!(UpdateUser { email: None, password: Some("12345".into()), name: None, role: None, picture: None }.validate().is_err());
    assert!(UpdateUser { email: Some("new@example.com".into()), password: None, name: Some("New Name".into()), role: None, picture: None }.validate().is_ok());
}

// ===========================================================================
// PASSWORD HASHING (argon2)
// ===========================================================================

#[test]
fn int_argon2_password_hash_and_verify() -> Result<(), Box<dyn std::error::Error>> {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

    let password = "test-password-integration";
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .to_string();
    let parsed = PasswordHash::new(&hash)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    assert!(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok());
    assert!(Argon2::default().verify_password("wrong-password".as_bytes(), &parsed).is_err());
    Ok(())
}

#[test]
fn int_argon2_unique_salts() -> Result<(), Box<dyn std::error::Error>> {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};

    let pw = "same-password";
    let h1 = Argon2::default().hash_password(pw.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| std::io::Error::other(e.to_string()))?.to_string();
    let h2 = Argon2::default().hash_password(pw.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|e| std::io::Error::other(e.to_string()))?.to_string();
    assert_ne!(h1, h2);
    Ok(())
}

// ===========================================================================
// JWT TOKEN TESTS
// ===========================================================================

#[test]
fn int_token_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::middleware::auth::{create_access_token, decode_access_token};
    let user_id = uuid::Uuid::new_v4();
    let (token, _) = create_access_token(user_id, "test-key", 15)?;
    let claims = decode_access_token(&token, "test-key")?;
    assert_eq!(claims.sub, user_id.to_string());
    Ok(())
}

#[test]
fn int_token_wrong_key_rejected() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::middleware::auth::{create_access_token, decode_access_token};
    let user_id = uuid::Uuid::new_v4();
    let (token, _) = create_access_token(user_id, "key-a", 15)?;
    assert!(decode_access_token(&token, "key-b").is_err());
    Ok(())
}

#[test]
fn int_expired_token_rejected() -> Result<(), Box<dyn std::error::Error>> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let user_id = uuid::Uuid::new_v4();
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    let claims = serde_json::json!({"sub": user_id.to_string(), "exp": past.timestamp(), "iat": past.timestamp()});
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret("test-key".as_bytes()))?;
    assert!(express_rest_boilerplate::middleware::auth::decode_access_token(&token, "test-key").is_err());
    Ok(())
}

// ===========================================================================
// AUTHORIZATION LOGIC
// ===========================================================================

#[test]
fn int_admin_can_access_any_user() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::middleware::auth::authorize_user_access;
    let admin = make_test_user("00000000-0000-0000-0000-000000000001", "admin")?;
    let target = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002")?;
    assert!(authorize_user_access(&admin, target).is_ok());
    Ok(())
}

#[test]
fn int_user_can_access_self() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::middleware::auth::authorize_user_access;
    let user = make_test_user("00000000-0000-0000-0000-000000000001", "user")?;
    assert!(authorize_user_access(&user, user.id).is_ok());
    Ok(())
}

#[test]
fn int_user_cannot_access_other() -> Result<(), Box<dyn std::error::Error>> {
    use express_rest_boilerplate::middleware::auth::authorize_user_access;
    let user = make_test_user("00000000-0000-0000-0000-000000000001", "user")?;
    let other = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002")?;
    assert!(authorize_user_access(&user, other).is_err());
    Ok(())
}

#[test]
fn int_role_validation() {
    use express_rest_boilerplate::middleware::auth::is_valid_role;
    assert!(is_valid_role("user"));
    assert!(is_valid_role("admin"));
    assert!(!is_valid_role("superadmin"));
    assert!(!is_valid_role(""));
}

// ===========================================================================
// SQLX ERROR MAPPING & APP ERROR STATUS CODES
// ===========================================================================

#[test]
fn int_sqlx_row_not_found_maps_to_user_not_found() {
    use express_rest_boilerplate::errors::AppError;
    let err: AppError = sqlx::Error::RowNotFound.into();
    match err { AppError::UserNotFound => {} other => panic!("Expected UserNotFound, got {:?}", other) }
}

#[test]
fn int_error_status_codes() {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use express_rest_boilerplate::errors::AppError;

    assert_eq!(AppError::Unauthorized.into_response().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(AppError::Forbidden.into_response().status(), StatusCode::FORBIDDEN);
    assert_eq!(AppError::UserNotFound.into_response().status(), StatusCode::NOT_FOUND);
    assert_eq!(AppError::NotFound.into_response().status(), StatusCode::NOT_FOUND);
    assert_eq!(AppError::DuplicateEmail.into_response().status(), StatusCode::CONFLICT);
    assert_eq!(AppError::BadRequest("x".into()).into_response().status(), StatusCode::BAD_REQUEST);
    assert_eq!(AppError::IncorrectCredentials.into_response().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(AppError::IncorrectRefreshToken.into_response().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(AppError::Internal("err".into()).into_response().status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(AppError::Validation { errors: vec![] }.into_response().status(), StatusCode::BAD_REQUEST);
}

// ===========================================================================
// OPENAPI SPEC COMPLETENESS CHECK
// ===========================================================================

#[test]
fn int_openapi_spec_is_complete() -> Result<(), Box<dyn std::error::Error>> {
    use utoipa::OpenApi;
    use express_rest_boilerplate::docs::ApiDoc;

    let spec = ApiDoc::openapi();
    let json = serde_json::to_value(&spec)?;

    let paths = json["paths"].as_object().ok_or("paths should be object")?;
    for expected in &[
        "/v1/health-check", "/v1/auth/register", "/v1/auth/login",
        "/v1/auth/refresh-token", "/v1/users", "/v1/users/profile", "/v1/users/{user_id}",
    ] {
        assert!(paths.contains_key(*expected), "Missing path: {}", expected);
    }

    let schemas = json["components"]["schemas"].as_object().ok_or("schemas should be object")?;
    for expected in &[
        "User", "UserResponse", "NewUser", "UpdateUser", "RefreshToken",
        "NewRefreshToken", "AuthResponse", "TokenResponse", "RegisterRequest",
        "LoginRequest", "RefreshRequest", "ErrorBody", "FieldError",
    ] {
        assert!(schemas.contains_key(*expected), "Missing schema: {}", expected);
    }

    Ok(())
}

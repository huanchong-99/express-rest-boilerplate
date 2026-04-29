//! Auth handlers - mirrors src/api/controllers/auth.controller.js
//!
//! Endpoints:
//!   POST /v1/auth/register      - Register a new user, return JWT + refresh token
//!   POST /v1/auth/login         - Authenticate, return JWT + refresh token
//!   POST /v1/auth/refresh-token - Exchange refresh token for new JWT

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::app_state::AppState;
use crate::errors::AppError;
use crate::extractors::ValidatedJson;
use crate::models::user::UserResponse;

/// Token response returned by auth endpoints.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: DateTime<Utc>,
}

/// Combined response for register/login: { token, user }
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: TokenResponse,
    pub user: UserResponse,
}

/// Request body for POST /v1/auth/register
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 6, max = 128))]
    pub password: String,
}

/// Request body for POST /v1/auth/login
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(max = 128))]
    pub password: String,
}

/// Request body for POST /v1/auth/refresh-token
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    pub refresh_token: String,
}

/// POST /v1/auth/register
#[utoipa::path(
    post,
    path = "/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = AuthResponse),
        (status = 400, description = "Validation error", body = crate::errors::ErrorBody),
        (status = 409, description = "Email already exists", body = crate::errors::ErrorBody),
    ),
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let (user, access_token, refresh_token, expires) =
        crate::services::auth_service::register_user(
            &state.pool,
            &state.config.token_signing_key,
            state.config.jwt_expiration_minutes,
            &body.email,
            &body.password,
        )
        .await?;

    let resp = AuthResponse {
        token: TokenResponse {
            token_type: "Bearer".into(),
            access_token,
            refresh_token,
            expires_in: expires,
        },
        user: UserResponse::from(user),
    };

    Ok((StatusCode::CREATED, Json(resp)))
}

/// POST /v1/auth/login
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Incorrect credentials", body = crate::errors::ErrorBody),
    ),
)]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (user, access_token, refresh_token, expires) =
        crate::services::auth_service::authenticate_user(
            &state.pool,
            &state.config.token_signing_key,
            state.config.jwt_expiration_minutes,
            &body.email,
            &body.password,
        )
        .await?;

    let resp = AuthResponse {
        token: TokenResponse {
            token_type: "Bearer".into(),
            access_token,
            refresh_token,
            expires_in: expires,
        },
        user: UserResponse::from(user),
    };

    Ok(Json(resp))
}

/// POST /v1/auth/refresh-token
#[utoipa::path(
    post,
    path = "/v1/auth/refresh-token",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = TokenResponse),
        (status = 401, description = "Incorrect refresh token", body = crate::errors::ErrorBody),
    ),
)]
pub async fn refresh(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let (_user, access_token, refresh_token, expires) =
        crate::services::auth_service::refresh_access_token(
            &state.pool,
            &state.config.token_signing_key,
            state.config.jwt_expiration_minutes,
            &body.email,
            &body.refresh_token,
        )
        .await?;

    let resp = TokenResponse {
        token_type: "Bearer".into(),
        access_token,
        refresh_token,
        expires_in: expires,
    };

    Ok(Json(resp))
}

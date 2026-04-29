//! Auth route stubs — mirrors src/api/routes/v1/auth.route.js
//!
//! Route group: /v1/auth
//!   POST /register      – Register a new user
//!   POST /login         – Authenticate and get tokens
//!   POST /refresh-token – Exchange refresh token for new access token
//!
//! TODO: POST /v1/auth/reset-password
//! TODO: POST /v1/auth/facebook
//! TODO: POST /v1/auth/google

use axum::http::StatusCode;

use crate::errors::AppError;

/// Placeholder: POST /v1/auth/register
#[allow(dead_code)]
pub async fn register_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: POST /v1/auth/login
#[allow(dead_code)]
pub async fn login_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: POST /v1/auth/refresh-token
#[allow(dead_code)]
pub async fn refresh_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

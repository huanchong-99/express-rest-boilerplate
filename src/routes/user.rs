//! User route stubs — mirrors src/api/routes/v1/user.route.js
//!
//! Route group: /v1/users
//!   GET    /              – List users (admin only)
//!   POST   /              – Create user (admin only)
//!   GET    /profile       – Get current user profile (any logged-in user)
//!   GET    /:userId       – Get user by ID (logged-in user or admin)
//!   PUT    /:userId       – Replace user (logged-in user or admin)
//!   PATCH  /:userId       – Update user (logged-in user or admin)
//!   DELETE /:userId       – Delete user (logged-in user or admin)

use axum::http::StatusCode;

use crate::errors::AppError;

/// Placeholder: GET /v1/users
#[allow(dead_code)]
pub async fn list_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: POST /v1/users
#[allow(dead_code)]
pub async fn create_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: GET /v1/users/profile
#[allow(dead_code)]
pub async fn profile_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: GET /v1/users/:userId
#[allow(dead_code)]
pub async fn get_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: PUT /v1/users/:userId
#[allow(dead_code)]
pub async fn replace_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: PATCH /v1/users/:userId
#[allow(dead_code)]
pub async fn update_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

/// Placeholder: DELETE /v1/users/:userId
#[allow(dead_code)]
pub async fn delete_stub() -> Result<StatusCode, AppError> {
    Err(AppError::NotFound)
}

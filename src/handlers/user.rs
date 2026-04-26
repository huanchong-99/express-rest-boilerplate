//! User handlers - mirrors src/api/controllers/user.controller.js
//!
//! Endpoints:
//!   GET    /v1/users          - List users (admin only)
//!   POST   /v1/users          - Create user (admin only)
//!   GET    /v1/users/profile  - Get current user profile
//!   GET    /v1/users/{id}     - Get user by ID
//!   PUT    /v1/users/{id}     - Replace user
//!   PATCH  /v1/users/{id}     - Update user
//!   DELETE /v1/users/{id}     - Delete user

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::app_state::AppState;
use crate::errors::AppError;
use crate::extractors::ValidatedJson;
use crate::middleware::auth::{authorize_user_access, AdminUser, LoggedUser, ROLES};
use crate::models::user::{NewUser, UpdateUser, UserResponse};

/// Query parameters for GET /v1/users
#[derive(Debug, Deserialize, Validate)]
pub struct ListUsersQuery {
    #[validate(range(min = 1))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<i64>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

/// GET /v1/users - List all users (admin only).
pub async fn list_users(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<ListUsersQuery>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(30);
    let role_filter = query.role.filter(|r| ROLES.contains(&r.as_str()));

    let users = crate::services::user_service::list_users(
        &state.pool,
        page,
        per_page,
        query.name,
        query.email,
        role_filter,
    )
    .await?;

    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

/// POST /v1/users - Create a new user (admin only).
pub async fn create_user(
    _admin: AdminUser,
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<NewUser>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    // Validate role if provided
    if let Some(ref role) = body.role {
        if !ROLES.contains(&role.as_str()) {
            return Err(AppError::BadRequest(format!(
                "role must be one of: {:?}",
                ROLES
            )));
        }
    }

    let user = crate::services::user_service::create_user(&state.pool, body).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// GET /v1/users/profile - Get current authenticated user's profile.
pub async fn get_profile(
    logged: LoggedUser,
) -> Result<Json<UserResponse>, AppError> {
    Ok(Json(UserResponse::from(logged.user)))
}

/// GET /v1/users/{user_id} - Get a user by ID.
pub async fn get_user(
    logged: LoggedUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;

    let user = crate::services::user_service::get_user(&state.pool, user_id).await?;
    Ok(Json(UserResponse::from(user)))
}

/// PUT /v1/users/{user_id} - Replace a user entirely.
pub async fn replace_user(
    logged: LoggedUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<NewUser>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;

    let is_admin = logged.user.role == "admin";
    let user =
        crate::services::user_service::replace_user(&state.pool, user_id, body, is_admin).await?;
    Ok(Json(UserResponse::from(user)))
}

/// PATCH /v1/users/{user_id} - Update a user partially.
pub async fn update_user(
    logged: LoggedUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;

    let is_admin = logged.user.role == "admin";
    let user =
        crate::services::user_service::update_user(&state.pool, user_id, body, is_admin).await?;
    Ok(Json(UserResponse::from(user)))
}

/// DELETE /v1/users/{user_id} - Delete a user.
pub async fn delete_user(
    logged: LoggedUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    authorize_user_access(&logged.user, user_id)?;

    crate::services::user_service::delete_user(&state.pool, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

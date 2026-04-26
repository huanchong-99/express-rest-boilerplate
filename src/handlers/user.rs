//! User handlers - mirrors src/api/controllers/user.controller.js

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::app_state::AppState;
use crate::errors::{AppError, FieldError};
use crate::extractors::{validate_to_app_error, ValidatedQuery};
use crate::middleware::auth::{authorize_user_access, is_valid_role, AdminUser, LoggedUser, ROLES};
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

/// GET /v1/users - Admin-only list users with filtering and pagination.
pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
    ValidatedQuery(query): ValidatedQuery<ListUsersQuery>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(30);

    if let Some(ref role) = query.role {
        if !is_valid_role(role) {
            return Err(AppError::Validation {
                errors: vec![FieldError::new(
                    "role",
                    "query",
                    vec![format!("\"role\" must be one of: {}", ROLES.join(", "))],
                )],
            });
        }
    }

    let users = crate::services::user_service::list_users(
        &state.pool,
        page,
        per_page,
        query.name,
        query.email,
        query.role,
    )
    .await?;

    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

/// POST /v1/users - Admin-only create user.
pub async fn create_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(body): Json<NewUser>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    validate_to_app_error(&body)?;

    if let Some(ref role) = body.role {
        if !is_valid_role(role) {
            return Err(AppError::Validation {
                errors: vec![FieldError::new(
                    "role",
                    "body",
                    vec![format!("\"role\" must be one of: {}", ROLES.join(", "))],
                )],
            });
        }
    }

    let user = crate::services::user_service::create_user(&state.pool, body).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// GET /v1/users/profile - Get logged-in user profile.
pub async fn get_profile(logged: LoggedUser) -> Result<Json<UserResponse>, AppError> {
    Ok(Json(UserResponse::from(logged.user)))
}

/// GET /v1/users/:user_id - Get user by ID (owner or admin).
pub async fn get_user(
    State(state): State<AppState>,
    logged: LoggedUser,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;
    let user = crate::services::user_service::get_user(&state.pool, user_id).await?;
    Ok(Json(UserResponse::from(user)))
}

/// PUT /v1/users/:user_id - Replace user entirely (owner or admin).
pub async fn replace_user(
    State(state): State<AppState>,
    logged: LoggedUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<NewUser>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;
    validate_to_app_error(&body)?;
    let is_admin = logged.user.role == "admin";

    if let Some(ref role) = body.role {
        if !is_valid_role(role) {
            return Err(AppError::Validation {
                errors: vec![FieldError::new(
                    "role",
                    "body",
                    vec![format!("\"role\" must be one of: {}", ROLES.join(", "))],
                )],
            });
        }
    }

    let user = crate::services::user_service::replace_user(&state.pool, user_id, body, is_admin).await?;
    Ok(Json(UserResponse::from(user)))
}

/// PATCH /v1/users/:user_id - Partial update (owner or admin).
pub async fn update_user(
    State(state): State<AppState>,
    logged: LoggedUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    authorize_user_access(&logged.user, user_id)?;
    validate_to_app_error(&body)?;
    let is_admin = logged.user.role == "admin";

    if let Some(ref role) = body.role {
        if !is_valid_role(role) {
            return Err(AppError::Validation {
                errors: vec![FieldError::new(
                    "role",
                    "body",
                    vec![format!("\"role\" must be one of: {}", ROLES.join(", "))],
                )],
            });
        }
    }

    let user = crate::services::user_service::update_user(&state.pool, user_id, body, is_admin).await?;
    Ok(Json(UserResponse::from(user)))
}

/// DELETE /v1/users/:user_id - Delete user (owner or admin).
pub async fn delete_user(
    State(state): State<AppState>,
    logged: LoggedUser,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    authorize_user_access(&logged.user, user_id)?;
    crate::services::user_service::delete_user(&state.pool, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

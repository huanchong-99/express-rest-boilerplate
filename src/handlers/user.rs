//! User handlers - mirrors src/api/controllers/user.controller.js

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::user::{NewUser, UpdateUser, UserResponse};

#[derive(Debug, serde::Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

pub async fn list_users(_query: Query<ListUsersQuery>) -> Result<Json<Vec<UserResponse>>, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn create_user(_body: Json<NewUser>) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn get_profile() -> Result<Json<UserResponse>, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn get_user(_path: Path<Uuid>) -> Result<Json<UserResponse>, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn replace_user(_path: Path<Uuid>, _body: Json<NewUser>) -> Result<Json<UserResponse>, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn update_user(_path: Path<Uuid>, _body: Json<UpdateUser>) -> Result<Json<UserResponse>, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

pub async fn delete_user(_path: Path<Uuid>) -> Result<StatusCode, AppError> {
    Err(AppError::Internal("Not yet implemented".into()))
}

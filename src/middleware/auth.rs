//! Auth middleware placeholder

use uuid::Uuid;

use crate::errors::AppError;
use crate::models::user::User;

pub const ADMIN: &str = "admin";
pub const LOGGED_USER: &str = "_loggedUser";

pub struct AuthMiddleware {
    pub user: User,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenClaims {
    pub exp: i64,
    pub iat: i64,
    pub sub: Uuid,
}

pub struct AppState;

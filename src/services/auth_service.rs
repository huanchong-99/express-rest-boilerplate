//! Auth service placeholder

use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::user::User;

pub async fn authenticate_user(_pool: &PgPool, _email: &str, _password: &str) -> Result<(User, String), AppError> { todo!() }
pub async fn refresh_access_token(_pool: &PgPool, _email: &str, _refresh_token: &str) -> Result<(User, String), AppError> { todo!() }

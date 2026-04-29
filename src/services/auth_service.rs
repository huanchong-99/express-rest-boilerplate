//! Auth service - mirrors auth.controller.js logic and RefreshToken statics.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::user::User;
use crate::middleware::auth::create_access_token;

/// Hash a password using argon2.
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hashing error: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a password against an argon2 hash.
fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Password hash parse error: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Register a new user and return (user, access_token, refresh_token, expires_at).
pub async fn register_user(
    pool: &PgPool,
    signing_key: &str,
    expiration_minutes: i64,
    email: &str,
    password: &str,
) -> Result<(User, String, String, chrono::DateTime<Utc>), AppError> {
    let hash = hash_password(password)?;

    let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password, role) VALUES ($1, $2, 'user') RETURNING *",
    )
    .bind(email.to_lowercase())
    .bind(&hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::DuplicateEmail;
            }
        }
        AppError::Internal(e.to_string())
    })?;

    let (access_token, expires) = create_access_token(user.id, signing_key, expiration_minutes)?;
    let new_refresh = create_refresh_token(&mut tx, &user).await?;

    tx.commit().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((user, access_token, new_refresh, expires))
}

/// Login: verify email + password, return tokens.
pub async fn authenticate_user(
    pool: &PgPool,
    signing_key: &str,
    expiration_minutes: i64,
    email: &str,
    password: &str,
) -> Result<(User, String, String, chrono::DateTime<Utc>), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email.to_lowercase())
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::IncorrectCredentials)?;

    let valid = verify_password(password, &user.password)?;

    if !valid {
        return Err(AppError::IncorrectCredentials);
    }

    let (access_token, expires) = create_access_token(user.id, signing_key, expiration_minutes)?;

    let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.to_string()))?;
    let new_refresh = create_refresh_token(&mut tx, &user).await?;
    tx.commit().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((user, access_token, new_refresh, expires))
}

/// Refresh: verify email + refresh token, consume old, issue new tokens.
pub async fn refresh_access_token(
    pool: &PgPool,
    signing_key: &str,
    expiration_minutes: i64,
    email: &str,
    refresh_token_str: &str,
) -> Result<(User, String, String, chrono::DateTime<Utc>), AppError> {
    let removed = sqlx::query_as::<_, crate::models::refresh_token::RefreshToken>(
        "DELETE FROM refresh_tokens WHERE user_email = $1 AND token = $2 RETURNING *",
    )
    .bind(email.to_lowercase())
    .bind(refresh_token_str)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .ok_or(AppError::IncorrectRefreshToken)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(removed.user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let (access_token, expires) = create_access_token(user.id, signing_key, expiration_minutes)?;

    let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.to_string()))?;
    let new_refresh = create_refresh_token(&mut tx, &user).await?;
    tx.commit().await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((user, access_token, new_refresh, expires))
}

/// Generate a new refresh token and persist it.
async fn create_refresh_token(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: &User,
) -> Result<String, AppError> {
    let token_value = format!(
        "{}.{}",
        user.id,
        Uuid::new_v4().to_string().replace('-', "")
    );
    let expires = Utc::now() + Duration::days(30);

    sqlx::query(
        "INSERT INTO refresh_tokens (token, user_id, user_email, expires) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token_value)
    .bind(user.id)
    .bind(&user.email)
    .bind(expires)
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(token_value)
}

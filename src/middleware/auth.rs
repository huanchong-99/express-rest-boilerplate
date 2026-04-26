//! Auth middleware - mirrors src/api/middlewares/auth.js

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::user::User;

/// User role constants, matching the original code.
pub const ADMIN: &str = "admin";
pub const LOGGED_USER: &str = "_loggedUser";
pub const ROLES: [&str; 2] = ["user", "admin"];

/// Claims encoded in the JWT.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenClaims {
    pub exp: i64,
    pub iat: i64,
    pub sub: String,
}

/// Generate a JWT access token for the given user id.
pub fn create_access_token(
    user_id: Uuid,
    jwt_secret: &str,
    expiration_minutes: i64,
) -> Result<(String, chrono::DateTime<Utc>), AppError> {
    let now = Utc::now();
    let expires = now + chrono::Duration::minutes(expiration_minutes);
    let claims = TokenClaims {
        sub: user_id.to_string(),
        exp: expires.timestamp(),
        iat: now.timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Token encode error: {e}")))?;

    Ok((token, expires))
}

/// Decode and validate a JWT token, returning the claims.
pub fn decode_access_token(token: &str, secret: &str) -> Result<TokenClaims, AppError> {
    let data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;

    Ok(data.claims)
}

/// The authenticated user extracted from the Authorization header.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user: User,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let pool = parts
                .extensions
                .get::<PgPool>()
                .cloned()
                .ok_or_else(|| AppError::Internal("PgPool not in extensions".into()))?;

            let config = parts
                .extensions
                .get::<AppConfig>()
                .cloned()
                .ok_or_else(|| AppError::Internal("AppConfig not in extensions".into()))?;

            let auth_header = parts
                .headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .ok_or(AppError::Unauthorized)?;

            let token = auth_header
                .strip_prefix("Bearer ")
                .ok_or(AppError::Unauthorized)?;

            let claims = decode_access_token(token, &config.jwt_secret)?;

            let user_id = Uuid::parse_str(&claims.sub)
                .map_err(|_| AppError::Unauthorized)?;

            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or(AppError::Unauthorized)?;

            Ok(AuthUser { user })
        })
    }
}

/// Admin-only guard: requires role = "admin".
#[derive(Debug, Clone)]
pub struct AdminUser {
    pub user: User,
}

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = AppError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let auth = AuthUser::from_request_parts(parts, state).await?;
            if auth.user.role != ADMIN {
                return Err(AppError::Forbidden);
            }
            Ok(AdminUser { user: auth.user })
        })
    }
}

/// Any logged-in user.
#[derive(Debug, Clone)]
pub struct LoggedUser {
    pub user: User,
}

impl<S: Send + Sync> FromRequestParts<S> for LoggedUser {
    type Rejection = AppError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, Self::Rejection>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let auth = AuthUser::from_request_parts(parts, state).await?;
            Ok(LoggedUser { user: auth.user })
        })
    }
}

/// Check if a logged-in user can access/modify a specific user_id resource.
pub fn authorize_user_access(caller: &User, target_id: Uuid) -> Result<(), AppError> {
    if caller.role == ADMIN || caller.id == target_id {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Check if a role string is valid.
pub fn is_valid_role(role: &str) -> bool {
    ROLES.contains(&role)
}

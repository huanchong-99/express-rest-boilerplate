//! User model – maps from the MongoDB `users` collection.
//!
//! Original Mongoose schema fields:
//!   email       – String, required, unique, lowercase, trimmed, email regex
//!   password    – String, required, min 6, max 128 chars (bcrypt hashed)
//!   name        – String, max 128, indexed, trimmed
//!   services    – { facebook: String, google: String }
//!   role        – String, enum ['user','admin'], default 'user'
//!   picture     – String, trimmed
//!   timestamps  – createdAt, updatedAt (auto by mongoose)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Database row for the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub role: String,
    pub picture: Option<String>,
    pub facebook_id: Option<String>,
    pub google_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new user.
/// Maps to the POST /v1/auth/register and POST /v1/users request bodies.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct NewUser {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 6, max = 128, message = "password must be 6–128 characters"))]
    pub password: String,
    #[validate(length(max = 128, message = "name must be at most 128 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 128, message = "role must be at most 128 characters"))]
    pub role: Option<String>,
}

/// Data for updating an existing user (PATCH).
/// All fields are optional – only provided fields will be updated.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUser {
    #[validate(email(message = "must be a valid email"))]
    pub email: Option<String>,
    #[validate(length(min = 6, max = 128, message = "password must be 6–128 characters"))]
    pub password: Option<String>,
    #[validate(length(max = 128, message = "name must be at most 128 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 128, message = "role must be at most 128 characters"))]
    pub role: Option<String>,
    pub picture: Option<String>,
}

/// Public user response – the `transform()` output from the original code.
/// Returned by all endpoints instead of the full User (hides password, etc.).
///
/// Original transform() returns: id, name, email, picture, role, createdAt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub name: Option<String>,
    pub email: String,
    pub picture: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            picture: user.picture,
            role: user.role,
            created_at: user.created_at,
        }
    }
}

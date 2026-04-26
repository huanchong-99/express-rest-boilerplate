//! RefreshToken model – maps from the MongoDB `refreshtokens` collection.
//!
//! Original Mongoose schema fields:
//!   token      – String, required, indexed
//!   userId     – ObjectId ref 'User', required
//!   userEmail  – String, ref 'User', required
//!   expires    – Date

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database row for the `refresh_tokens` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub token: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub expires: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new refresh token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRefreshToken {
    pub token: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub expires: Option<DateTime<Utc>>,
}

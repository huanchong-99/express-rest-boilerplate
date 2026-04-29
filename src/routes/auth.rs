//! Auth routes — mirrors src/api/routes/v1/auth.route.js
//!
//! Route group: /v1/auth
//!   POST /register      – Register a new user
//!   POST /login         – Authenticate and get tokens
//!   POST /refresh-token – Exchange refresh token for new access token
//!
//! TODO: POST /v1/auth/reset-password
//! TODO: POST /v1/auth/facebook
//! TODO: POST /v1/auth/google

use axum::routing::post;
use axum::Router;

use crate::app_state::AppState;
use crate::handlers;

/// Build the /v1/auth route group.
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/refresh-token", post(handlers::auth::refresh))
}

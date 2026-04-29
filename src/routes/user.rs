//! User routes — mirrors src/api/routes/v1/user.route.js
//!
//! Route group: /v1/users
//!   GET    /              – List users (admin only)
//!   POST   /              – Create user (admin only)
//!   GET    /profile       – Get current user profile (any logged-in user)
//!   GET    /:userId       – Get user by ID (logged-in user or admin)
//!   PUT    /:userId       – Replace user (logged-in user or admin)
//!   PATCH  /:userId       – Update user (logged-in user or admin)
//!   DELETE /:userId       – Delete user (logged-in user or admin)

use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;
use crate::handlers;

/// Build the /v1/users route group.
pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::user::list_users).post(handlers::user::create_user),
        )
        .route("/profile", get(handlers::user::get_profile))
        .route(
            "/:user_id",
            get(handlers::user::get_user)
                .put(handlers::user::replace_user)
                .patch(handlers::user::update_user)
                .delete(handlers::user::delete_user),
        )
}

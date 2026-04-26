//! Library crate for express-rest-boilerplate.
//!
//! Re-exports key types and provides `create_app()` for integration testing.

pub mod app_state;
pub mod config;
pub mod db;
pub mod docs;
pub mod errors;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod schema;
pub mod services;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::app_state::AppState;
use crate::docs::ApiDoc;

/// Build the complete Axum application router.
///
/// This is the single place where all routes, middleware, and state are wired together.
/// Used by both `main.rs` and integration tests.
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .route("/v1/health-check", get(handlers::health::health_check))
        .route("/v1/auth/register", post(handlers::auth::register))
        .route("/v1/auth/login", post(handlers::auth::login))
        .route("/v1/auth/refresh-token", post(handlers::auth::refresh))
        .route(
            "/v1/users",
            get(handlers::user::list_users).post(handlers::user::create_user),
        )
        .route("/v1/users/profile", get(handlers::user::get_profile))
        .route(
            "/v1/users/{user_id}",
            get(handlers::user::get_user)
                .put(handlers::user::replace_user)
                .patch(handlers::user::update_user)
                .delete(handlers::user::delete_user),
        )
        .layer(axum::Extension(state.pool.clone()))
        .layer(axum::Extension(state.config.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

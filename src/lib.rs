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
pub mod routes;
pub mod schema;
pub mod services;
#[cfg(test)]
pub mod test_utils;

use axum::routing::get;
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
        .nest("/v1/auth", routes::auth::auth_routes())
        .nest("/v1/users", routes::user::user_routes())
        .layer(axum::Extension(state.pool.clone()))
        .layer(axum::Extension(state.config.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

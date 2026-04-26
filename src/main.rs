mod app_state;
mod config;
mod db;
mod errors;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod schema;
mod services;

use std::process;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;

#[tokio::main]
async fn main() {
    if let Err(e) = run_server().await {
        tracing::error!("Fatal error: {e}");
        process::exit(1);
    }
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "express_rest_boilerplate=debug,tower_http=debug".into()),
        )
        .init();

    let app_config = config::AppConfig::from_env()?;
    let port = app_config.port;
    let host = app_config.host.clone();

    let pool = db::create_pool(&app_config.database_url).await?;

    db::run_migrations(&pool).await?;

    tracing::info!("Database connected and migrations applied");

    let state = AppState {
        pool: pool.clone(),
        config: app_config.clone(),
    };

    let app = Router::new()
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
        .layer(axum::Extension(pool))
        .layer(axum::Extension(app_config))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server started on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

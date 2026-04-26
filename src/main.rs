mod config;
mod db;
mod errors;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod schema;
mod services;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "express_rest_boilerplate=debug,tower_http=debug".into()),
        )
        .init();

    let app_config = config::AppConfig::from_env();
    let port = app_config.port;
    let host = app_config.host.clone();

    let pool = db::create_pool(&app_config.database_url)
        .await
        .expect("Failed to create database pool");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database connected and migrations applied");

    let app = Router::new()
        .route("/v1/health-check", get(handlers::health::health_check))
        .route("/v1/auth/register", post(handlers::auth::register))
        .route("/v1/auth/login", post(handlers::auth::login))
        .route("/v1/auth/refresh-token", post(handlers::auth::refresh_token))
        .route("/v1/users", get(handlers::user::list_users).post(handlers::user::create_user))
        .route("/v1/users/profile", get(handlers::user::get_profile))
        .route(
            "/v1/users/{user_id}",
            get(handlers::user::get_user)
                .put(handlers::user::replace_user)
                .patch(handlers::user::update_user)
                .delete(handlers::user::delete_user),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("Server started on {}", addr);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

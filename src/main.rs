use std::process;

use express_rest_boilerplate::app_state::AppState;
use express_rest_boilerplate::config;
use express_rest_boilerplate::create_app;
use express_rest_boilerplate::db;

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

    let app = create_app(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server started on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

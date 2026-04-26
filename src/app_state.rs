//! Shared application state passed to all Axum handlers via State extractor.

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::config::AppConfig;

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

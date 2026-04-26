//! Shared application state.

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::config::AppConfig;

/// Shared application state available to all handlers via State extractor.
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

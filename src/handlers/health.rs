//! Health check handler

use axum::http::StatusCode;
use axum::Json;

/// GET /v1/health-check
#[utoipa::path(
    get,
    path = "/v1/health-check",
    responses(
        (status = 200, description = "Service is healthy", body = String),
    ),
)]
pub async fn health_check() -> (StatusCode, Json<&'static str>) {
    (StatusCode::OK, Json("OK"))
}

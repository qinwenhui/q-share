use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub uptime_secs: u64,
}

pub async fn health() -> Json<HealthResponse> {
    static START: once_cell::sync::Lazy<std::time::Instant> =
        once_cell::sync::Lazy::new(std::time::Instant::now);
    Json(HealthResponse {
        status: "ok",
        uptime_secs: START.elapsed().as_secs(),
    })
}

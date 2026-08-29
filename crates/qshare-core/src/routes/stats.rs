//! GET /api/stats — live server counters for the GUI / TUI dashboard.

use axum::extract::State;
use axum::Json;

use crate::state::AppState;

pub async fn stats(State(state): State<AppState>) -> Json<crate::stats::StatsSnapshot> {
    Json(state.stats.snapshot())
}

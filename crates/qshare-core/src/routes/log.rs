//! GET /api/log — tail of the server's rolling live log.
//!
//! `?tail=N` (default 200) returns the last N lines. Used by the GUI to
//! render the bottom log panel without scraping stderr.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    #[serde(default = "default_tail")]
    pub tail: usize,
}

fn default_tail() -> usize {
    200
}

pub async fn log(
    State(state): State<AppState>,
    Query(q): Query<LogQuery>,
) -> Json<serde_json::Value> {
    let lines = state.log.tail(q.tail.min(1000));
    Json(serde_json::json!({
        "lines": lines,
        "total_cached": state.log.tail(usize::MAX).len(),
    }))
}

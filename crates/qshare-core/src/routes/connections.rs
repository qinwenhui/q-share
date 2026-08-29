//! GET /api/connections — list of currently active WebSocket clients.
//!
//! Returned shape is intentionally flat — the GUI renders this in a table,
//! no client-side transformation needed.

use axum::extract::State;
use axum::Json;

use crate::state::AppState;

pub async fn connections(State(state): State<AppState>) -> Json<serde_json::Value> {
    let conns = state.connections.snapshot();
    Json(serde_json::json!({
        "count": conns.len(),
        "connections": conns,
    }))
}

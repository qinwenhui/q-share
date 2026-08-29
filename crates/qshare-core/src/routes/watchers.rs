//! GET /api/watchers — currently armed per-directory watchers.
//!
//! A new entry appears the moment a client subscribes to a path via WS;
//! it disappears 30 s after the last subscriber leaves.

use axum::extract::State;
use axum::Json;

use crate::state::AppState;

pub async fn watchers(State(state): State<AppState>) -> Json<serde_json::Value> {
    let snap = state.watchers.snapshot();
    Json(serde_json::json!({
        "count": snap.len(),
        "watchers": snap.into_iter().map(|(path, id, subs)| {
            serde_json::json!({
                "id": id,
                "path": path,
                "subscribers": subs,
            })
        }).collect::<Vec<_>>(),
    }))
}

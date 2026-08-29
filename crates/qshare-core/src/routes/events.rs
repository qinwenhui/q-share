//! Legacy SSE endpoint (`/api/events`) — kept for backwards compat with the
//! old frontend. New clients should use the WebSocket endpoint at `/ws`.
//!
//! In the new model there is no global FS watcher, so this endpoint
//! receives nothing on its own. We still emit a heartbeat every 30 s so
//! existing EventSource clients don't time out.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};

use crate::state::AppState;
use crate::watcher::now_secs;

pub async fn events(
    _state: State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let hb = async_stream::stream! {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.tick().await; // skip immediate
        loop {
            tick.tick().await;
            let payload = serde_json::json!({ "ts": now_secs() }).to_string();
            yield Ok::<_, Infallible>(
                Event::default()
                    .event("heartbeat")
                    .data(payload)
            );
        }
    };
    Sse::new(hb).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

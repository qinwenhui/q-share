//! Tower middleware that updates [`Stats`] for every request:
//! - Increments active counter on entry, decrements on drop.
//! - Counts **actual bytes streamed** to the client (not Content-Length).
//!   This is important for streaming responses like video — when the browser
//!   cancels mid-playback (e.g. user scrubs the slider), bytes the server
//!   *promised* but never actually sent don't pollute the counter.
//! - Marks 4xx/5xx as errors.

use std::sync::Arc;
use std::task::{Context, Poll};

use axum::{
    body::Body,
    extract::State,
    http::{header::USER_AGENT, Request},
    middleware::Next,
    response::Response,
};
use bytes::Bytes;
use futures::Stream;

use crate::state::AppState;
use crate::stats::Stats;

/// User-Agent string used by the GUI's dashboard poller. Real browsers send
/// their own UA (Chrome / Safari / Firefox / …), so this filter can only
/// ever exclude our own internal polling — it cannot mis-attribute a real
/// user's download as "internal traffic".
const GUI_POLL_UA_PREFIX: &str = "qshare";

/// Wrap a request, record lifecycle metrics, return the response with a
/// body that counts bytes as they flow out.
pub async fn track(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let stats = state.stats.clone();
    stats.on_request_start();
    let method = req.method().clone();
    // Full URI (path + query) — an error line like "GET /api/thumb → 422"
    // needs the query to say *which* file failed.
    let path = req.uri().to_string();

    // Snapshot the UA *before* we hand the request to the inner stack —
    // `next.run(req)` consumes it. We only need this for the bytes-served
    // exception below.
    let is_gui_poll = req
        .headers()
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.starts_with(GUI_POLL_UA_PREFIX));

    // RAII: decrement on any return path (panic, early-return, success).
    struct Guard(Arc<Stats>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.on_request_end();
        }
    }
    let _guard = Guard(stats.clone());

    let response = next.run(req).await;

    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        stats.record_error();
        // Surface request errors in the live log too, not just the counter —
        // otherwise the dashboard's `errors` number climbs with no explanation
        // in the log panel (e.g. a 404 for a file deleted mid-browse, or a 422
        // from a corrupt image).
        state
            .log
            .push("warn", format!("{method} {path} → {}", status.as_u16()));
    }

    // Wrap the body so we count actual bytes the client receives, not the
    // Content-Length the server promised. Streaming responses (Range video,
    // SSE) would otherwise inflate `bytes_served` on every mid-stream cancel.
    //
    // Exception: requests from the GUI's own dashboard poller carry
    // `User-Agent: qshare` (see crates/qshare-gui/src/main.rs). The 1 Hz
    // poll on /api/stats + /api/connections + /api/log + /api/watchers would
    // otherwise add ~14 KB/min of zero-information "transfer" to the
    // counter, polluting the real user-facing metric. Active-connection
    // count and error count are still tracked above — only the byte counter
    // is skipped.
    let (parts, body) = response.into_parts();
    let counted = if is_gui_poll {
        body
    } else {
        Body::from_stream(CountingStream {
            inner: body.into_data_stream(),
            stats: stats.clone(),
        })
    };
    let response = Response::from_parts(parts, counted);

    tracing::trace!(%method, %path, %status, gui_poll = is_gui_poll, "request done");
    response
}

/// Stream wrapper that increments `Stats::add_bytes` for every chunk yielded.
pub struct CountingStream<S> {
    inner: S,
    stats: Arc<Stats>,
}

impl<S> Stream for CountingStream<S>
where
    S: Stream<Item = Result<Bytes, axum::Error>> + Unpin,
{
    type Item = Result<Bytes, axum::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if !chunk.is_empty() {
                    self.stats.add_bytes(chunk.len());
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            other => other,
        }
    }
}

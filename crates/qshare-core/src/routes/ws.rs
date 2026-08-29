//! WebSocket endpoint `/ws` — single bidirectional channel for everything
//! push-oriented.
//!
//! ## Protocol
//!
//! All frames are JSON text frames.
//!
//! Client → server:
//! - `{ "op": "ping" }` — keep-alive; server replies with `{ "type": "pong" }`.
//! - `{ "op": "watch", "path": "/photos/summer" }` — start receiving FS
//!   events for that subtree. Server replies with `{ "type": "watching",
//!   "path": ..., "watcher_id": ... }`. Idempotent.
//! - `{ "op": "unwatch", "path": "/photos/summer" }` — stop receiving
//!   events. Server replies with `{ "type": "unwatched", ... }`.
//! - `{ "op": "bye" }` — graceful close.
//!
//! Server → client (push):
//! - `{ "type": "welcome", "id": "<uuid>", "server_version": ... }` — once.
//! - `{ "type": "fs-event", "path": "/foo", "events": [ ...FsEvent... ] }`
//!   — pushed when a watched dir changes.
//! - `{ "type": "stats", "active": N, "bytes_served": B, "errors": E }`
//!   — every 2 s.
//! - `{ "type": "log", "level": "...", "msg": "..." }` — server-side events
//!   the GUI cares about (connects, watcher armed, errors).
//! - `{ "type": "pong" }` — reply to client `ping`.

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::{header::USER_AGENT, HeaderMap};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc::{self, error::TrySendError};

use crate::state::AppState;
use crate::watcher::FsEvent;

/// Per-connection outgoing mailbox capacity. Producers use [`try_push`] which
/// never blocks and never buffers without bound: a full mailbox means the
/// client isn't draining (stalled/broken socket), so the message is dropped.
/// Live snapshot feeds (stats, fs-events) tolerate drops — the browser just
/// re-lists on the next event. 256 is well above the steady-state rate (stats
/// every 2 s + debounced fs-events), so drops only happen for a genuinely
/// stuck client.
const OUTBOX_CAPACITY: usize = 256;

/// Queue `msg` for a connection without blocking. Returns `true` when the
/// receiver is gone and the producer should stop; a full mailbox returns
/// `false` (message dropped, never buffered).
fn try_push(tx: &mpsc::Sender<Message>, msg: Message) -> bool {
    matches!(tx.try_send(msg), Err(TrySendError::Closed(_)))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ClientMsg {
    Ping,
    Watch {
        path: String,
        #[serde(default)]
        recursive: Option<bool>,
    },
    Unwatch {
        path: String,
    },
    Bye,
}

pub async fn ws(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    ws.on_upgrade(move |socket| handle(socket, state, addr.ip(), user_agent))
}

async fn handle(socket: WebSocket, state: AppState, ip: IpAddr, user_agent: String) {
    let (id, info) = state.connections.register(ip, user_agent.clone());
    state.log.push(
        "info",
        format!(
            "[+] {} connected  ua=\"{}\"",
            info.ip,
            truncate(&info.user_agent, 40)
        ),
    );

    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(OUTBOX_CAPACITY);

    // ── Welcome ─────────────────────────────────────────────────────────
    let welcome = json!({
        "type": "welcome",
        "id": id,
        "server_version": env!("CARGO_PKG_VERSION"),
        "ip": info.ip,
    });
    let _ = try_push(&out_tx, Message::Text(welcome.to_string().into()));

    // ── State shared with sub-tasks ─────────────────────────────────────
    let my_paths: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // ── Reader task: incoming frames ────────────────────────────────────
    let reader_id = id.clone();
    let reader_state = state.clone();
    let reader_paths = Arc::clone(&my_paths);
    let reader_tx = out_tx.clone();
    let reader = tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            let msg = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(Message::Binary(_)) => continue,
            };
            let parsed: Result<ClientMsg, _> = serde_json::from_str(&msg);
            let parsed = match parsed {
                Ok(m) => m,
                Err(e) => {
                    tracing::debug!("bad ws frame from {reader_id}: {e}");
                    continue;
                }
            };
            match parsed {
                ClientMsg::Ping => {
                    let _ = try_push(
                        &reader_tx,
                        Message::Text(json!({ "type": "pong" }).to_string().into()),
                    );
                }
                ClientMsg::Watch { path, recursive } => {
                    let rec = recursive.unwrap_or_else(|| !is_top_level(&path));
                    match reader_state.watchers.subscribe(&path, rec, &reader_id) {
                        Ok((w, _rx)) => {
                            reader_state
                                .connections
                                .set_watching(&reader_id, path.clone());
                            reader_paths.lock().insert(path.clone());
                            reader_state
                                .log
                                .push("info", format!("[→] {} watch {}", short(&reader_id), path));
                            let _ = try_push(
                                &reader_tx,
                                Message::Text(
                                    json!({
                                        "type": "watching",
                                        "path": path,
                                        "watcher_id": w.id,
                                        "recursive": rec,
                                    })
                                    .to_string()
                                    .into(),
                                ),
                            );
                        }
                        Err(e) => {
                            reader_state.log.push(
                                "warn",
                                format!("[!] {} watch {} failed: {}", short(&reader_id), path, e),
                            );
                            let _ = try_push(
                                &reader_tx,
                                Message::Text(
                                    json!({
                                        "type": "error",
                                        "op": "watch",
                                        "message": e.to_string(),
                                    })
                                    .to_string()
                                    .into(),
                                ),
                            );
                        }
                    }
                }
                ClientMsg::Unwatch { path } => {
                    reader_state.watchers.unsubscribe(&path, &reader_id);
                    reader_state
                        .connections
                        .set_watching(&reader_id, String::new());
                    reader_paths.lock().remove(&path);
                    reader_state.log.push(
                        "info",
                        format!("[←] {} unwatch {}", short(&reader_id), path),
                    );
                    let _ = try_push(
                        &reader_tx,
                        Message::Text(
                            json!({ "type": "unwatched", "path": path })
                                .to_string()
                                .into(),
                        ),
                    );
                }
                ClientMsg::Bye => break,
            }
        }
    });

    // ── FS-event forwarder: one per currently watched path ──────────────
    let fwd_id = id.clone();
    let fwd_state = state.clone();
    let fwd_paths = Arc::clone(&my_paths);
    let fwd_tx = out_tx.clone();
    let forwarder = tokio::spawn(async move {
        // Per-path receiver handles. We add when a path appears in my_paths,
        // remove when it disappears. Spawning a fresh task per forwarder
        // keeps each FsEvent channel isolated.
        let mut handles: Vec<(String, tokio::task::JoinHandle<()>)> = Vec::new();
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        tick.tick().await; // skip immediate
        loop {
            tick.tick().await;
            let cur: HashSet<String> = fwd_paths.lock().clone();
            handles.retain(|(_, h)| !h.is_finished());
            let known: HashSet<String> = handles.iter().map(|(p, _)| p.clone()).collect();
            for path in cur.difference(&known) {
                let rx = match fwd_state
                    .watchers
                    .subscribe(path, !is_top_level(path), &fwd_id)
                {
                    Ok((_, rx)) => rx,
                    Err(_) => continue,
                };
                let tx = fwd_tx.clone();
                let cid = fwd_id.clone();
                let p = path.clone();
                let h = tokio::spawn(async move {
                    let mut rx = rx;
                    while let Ok(ev) = rx.recv().await {
                        let payload = json!({
                            "type": "fs-event",
                            "path": p,
                            "events": flatten(ev),
                        });
                        if try_push(&tx, Message::Text(payload.to_string().into())) {
                            break;
                        }
                    }
                    tracing::trace!("fs-event forwarder for {cid} ended");
                });
                handles.push((path.clone(), h));
            }
            // Abort forwarders for paths that are no longer in `cur`.
            handles.retain(|(p, h)| {
                if !cur.contains(p) {
                    h.abort();
                    false
                } else {
                    true
                }
            });
        }
    });

    // ── Stats pusher: every 2 s ─────────────────────────────────────────
    let stats_state = state.clone();
    let stats_tx = out_tx.clone();
    let stats_task = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        tick.tick().await; // skip immediate
        loop {
            tick.tick().await;
            let snap = stats_state.stats.snapshot();
            let msg = json!({
                "type": "stats",
                "active": snap.active,
                "bytes_served": snap.bytes_served,
                "errors": snap.errors,
            });
            if try_push(&stats_tx, Message::Text(msg.to_string().into())) {
                break;
            }
        }
    });

    // ── Writer: drain out_rx into the sink ──────────────────────────────
    let writer = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // ── Wait for reader (client disconnect) ─────────────────────────────
    let _ = reader.await;

    // Cleanup.
    let paths = my_paths.lock().clone();
    for p in paths {
        state.watchers.unsubscribe(&p, &id);
    }
    state.connections.unregister(&id, "client-close");
    state
        .log
        .push("info", format!("[-] {} disconnected", info.ip));

    forwarder.abort();
    stats_task.abort();
    // writer will exit naturally when out_rx is dropped below.
    drop(out_tx);
    let _ = writer.await;
}

fn is_top_level(path: &str) -> bool {
    let t = path.trim_start_matches('/');
    t.is_empty() || !t.contains('/')
}

fn flatten(ev: FsEvent) -> Vec<Value> {
    match ev {
        FsEvent::Batch(events) => events
            .into_iter()
            .filter_map(|e| serde_json::to_value(e).ok())
            .collect(),
        other => vec![serde_json::to_value(other).unwrap_or(Value::Null)],
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

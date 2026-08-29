//! Active WebSocket connection registry.
//!
//! Every browser / GUI client that connects to `/ws` gets one [`ConnInfo`]
//! stored in [`ConnectionRegistry`]. The registry is the source of truth
//! for the GUI's "active connections" panel and the per-connection byte
//! counters that show up in the dashboard.
//!
//! Disconnects are detected two ways:
//! 1. Explicit (WebSocket close frame) — handler calls [`unregister`].
//! 2. Stale (no activity for > STALE_SECS) — janitor task sweeps periodically.
//!
//! The registry also broadcasts every (register / unregister / event) tuple
//! on a tokio broadcast channel so the live-log subsystem and the GUI
//! stats loop can react in real time without polling.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Public-facing snapshot of one active connection. Sent over JSON.
#[derive(Debug, Clone, Serialize)]
pub struct ConnInfo {
    pub id: String,
    pub ip: String,
    /// `User-Agent` header (browser / qshare / curl / …).
    pub user_agent: String,
    /// Currently watched URL-path (e.g. `/photos/summer`). Empty = idle.
    pub watching: String,
    pub bytes_sent: u64,
    pub uptime_secs: u64,
    pub last_seen_unix_ms: u64,
}

/// Internal mutable record. Lives in the [`ConnectionRegistry`]'s map;
/// the public [`ConnInfo`] is a `Clone` of the snapshot.
#[derive(Debug)]
pub(crate) struct ConnRecord {
    pub info: ConnInfo,
    /// When the connection opened. Used to compute uptime.
    pub opened_at: Instant,
}

/// Things the registry may broadcast to subscribers (e.g. live log).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConnEvent {
    Opened {
        id: String,
        ip: String,
        user_agent: String,
        ts: u64,
    },
    Closed {
        id: String,
        reason: String,
        ts: u64,
    },
    Subscribed {
        id: String,
        path: String,
        ts: u64,
    },
    Unsubscribed {
        id: String,
        path: String,
        ts: u64,
    },
}

/// How long without any traffic before a connection is considered dead.
/// Generous (90 s) — browsers behind slow proxies may sit idle briefly.
pub const STALE_SECS: u64 = 90;

#[derive(Clone)]
pub struct ConnectionRegistry {
    inner: Arc<RwLock<HashMap<String, ConnRecord>>>,
    /// Broadcast channel for live subscribers (GUI log, future audit trail).
    /// 1024-slot ring; receivers that lag lose old events (acceptable for logs).
    pub events: broadcast::Sender<ConnEvent>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        let (events, _rx) = broadcast::channel(1024);
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            events,
        }
    }

    /// Register a new connection; returns its assigned id and the
    /// initial [`ConnInfo`] (handy for the welcome message).
    pub fn register(&self, ip: IpAddr, user_agent: String) -> (String, ConnInfo) {
        let id = Uuid::new_v4().to_string();
        let opened_at = Instant::now();
        let now_unix_ms = unix_ms();
        let info = ConnInfo {
            id: id.clone(),
            ip: ip.to_string(),
            user_agent,
            watching: String::new(),
            bytes_sent: 0,
            uptime_secs: 0,
            last_seen_unix_ms: now_unix_ms,
        };
        let record = ConnRecord {
            info: info.clone(),
            opened_at,
        };
        self.inner.write().insert(id.clone(), record);

        let _ = self.events.send(ConnEvent::Opened {
            id: id.clone(),
            ip: info.ip.clone(),
            user_agent: info.user_agent.clone(),
            ts: unix_secs(),
        });
        (id, info)
    }

    /// Remove a connection (called on WebSocket close or on stale-sweep).
    /// `reason` is short and human-readable ("client-close", "stale-timeout").
    pub fn unregister(&self, id: &str, reason: &str) {
        let removed = self.inner.write().remove(id);
        if let Some(rec) = removed {
            let _ = self.events.send(ConnEvent::Closed {
                id: id.to_string(),
                reason: reason.to_string(),
                ts: unix_secs(),
            });
            tracing::debug!(
                id,
                watched = %rec.info.watching,
                bytes = rec.info.bytes_sent,
                "connection closed: {reason}"
            );
        }
    }

    /// Set / clear the path this connection is currently subscribed to.
    /// `path` is the URL-style path (e.g. `/photos`); empty string = none.
    pub fn set_watching(&self, id: &str, path: String) {
        let mut guard = self.inner.write();
        if let Some(rec) = guard.get_mut(id) {
            let prev = std::mem::replace(&mut rec.info.watching, path.clone());
            rec.info.last_seen_unix_ms = unix_ms();
            if prev != path {
                let _ = self.events.send(if path.is_empty() {
                    ConnEvent::Unsubscribed {
                        id: id.to_string(),
                        path: prev,
                        ts: unix_secs(),
                    }
                } else {
                    ConnEvent::Subscribed {
                        id: id.to_string(),
                        path,
                        ts: unix_secs(),
                    }
                });
            }
        }
    }

    /// Add to the per-connection byte counter (cheap, called from middleware).
    pub fn add_bytes(&self, id: &str, n: u64) {
        if n == 0 {
            return;
        }
        let mut guard = self.inner.write();
        if let Some(rec) = guard.get_mut(id) {
            rec.info.bytes_sent = rec.info.bytes_sent.saturating_add(n);
            rec.info.last_seen_unix_ms = unix_ms();
        }
    }

    /// Touch the last-seen timestamp without changing anything else.
    /// Cheap fast-path used by middleware to avoid stalls on the read lock.
    pub fn touch(&self, id: &str) {
        let mut guard = self.inner.write();
        if let Some(rec) = guard.get_mut(id) {
            rec.info.last_seen_unix_ms = unix_ms();
        }
    }

    /// Snapshot all current connections. Used by `/api/connections`.
    pub fn snapshot(&self) -> Vec<ConnInfo> {
        let guard = self.inner.read();
        let now = Instant::now();
        guard
            .values()
            .map(|rec| {
                let mut info = rec.info.clone();
                info.uptime_secs = now.duration_since(rec.opened_at).as_secs();
                info
            })
            .collect()
    }

    /// Drop stale connections (no traffic for > STALE_SECS). Returns the
    /// number of connections evicted — handy for log messages.
    pub fn sweep_stale(&self) -> usize {
        let cutoff_ms = unix_ms().saturating_sub(STALE_SECS * 1000);
        let stale: Vec<String> = {
            let guard = self.inner.read();
            guard
                .iter()
                .filter(|(_, r)| r.info.last_seen_unix_ms < cutoff_ms)
                .map(|(id, _)| id.clone())
                .collect()
        };
        let n = stale.len();
        for id in &stale {
            self.unregister(id, "stale-timeout");
        }
        n
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// One line in the server's rolling live log. GUI polls `/api/log` for these.
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub ts_ms: u64,
    /// Coarse level for icon colouring: "info" | "warn" | "error" | "system".
    pub level: String,
    pub msg: String,
}

/// Append-only ring buffer of recent log lines. Capacity 500 — at 1 Hz that's
/// 8 minutes of history; longer history isn't useful for a live dashboard.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<RwLock<Vec<LogLine>>>,
    cap: usize,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self::with_capacity(500)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::with_capacity(cap))),
            cap,
        }
    }

    pub fn push(&self, level: &str, msg: impl Into<String>) {
        let line = LogLine {
            ts_ms: unix_ms(),
            level: level.to_string(),
            msg: msg.into(),
        };
        let mut g = self.inner.write();
        if g.len() == self.cap {
            g.remove(0);
        }
        g.push(line);
    }

    /// Snapshot the last `n` lines (newest last).
    pub fn tail(&self, n: usize) -> Vec<LogLine> {
        let g = self.inner.read();
        let start = g.len().saturating_sub(n);
        g[start..].to_vec()
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Background janitor that periodically sweeps stale connections.
/// Spawn one per process — cheap (one timer + one mutex sweep every 30 s).
pub fn spawn_janitor(registry: ConnectionRegistry) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.tick().await; // skip immediate
        loop {
            tick.tick().await;
            let n = registry.sweep_stale();
            if n > 0 {
                tracing::info!("janitor swept {n} stale connection(s)");
            }
        }
    });
}

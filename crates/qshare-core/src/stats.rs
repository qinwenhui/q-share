//! Live server statistics — atomic counters updated by middleware.

use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Debug, Default)]
pub struct Stats {
    /// Active connections (in-flight requests).
    active: AtomicI64,
    /// Total bytes sent in response bodies across the lifetime of the server.
    bytes_served: AtomicI64,
    /// Total HTTP error responses (4xx/5xx) since startup.
    errors: AtomicI64,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_request_start(&self) {
        self.active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn on_request_end(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn add_bytes(&self, n: usize) {
        if n > 0 {
            self.bytes_served.fetch_add(n as i64, Ordering::Relaxed);
        }
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            active: self.active.load(Ordering::Relaxed).max(0) as u64,
            bytes_served: self.bytes_served.load(Ordering::Relaxed).max(0) as u64,
            errors: self.errors.load(Ordering::Relaxed).max(0) as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct StatsSnapshot {
    pub active: u64,
    pub bytes_served: u64,
    pub errors: u64,
}

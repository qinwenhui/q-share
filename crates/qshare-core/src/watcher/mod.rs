pub mod registry;
pub use registry::{spawn_sweeper, WatchedDir, WatcherRegistry};

use serde::Serialize;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

/// A filesystem change observed and debounced by the watcher.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FsEvent {
    Created {
        path: PathBuf,
        is_dir: bool,
        ts: u64,
    },
    Modified {
        path: PathBuf,
        is_dir: bool,
        ts: u64,
    },
    Removed {
        path: PathBuf,
        is_dir: bool,
        ts: u64,
    },
    Renamed {
        from: PathBuf,
        to: PathBuf,
        ts: u64,
    },
    /// Batch of events fired in a single debounce window — used internally
    /// to ferry events from the watcher thread to subscribers; receivers
    /// can flatten or just look at the latest.
    Batch(Vec<FsEvent>),
}

impl FsEvent {
    pub fn ts(&self) -> u64 {
        match self {
            FsEvent::Created { ts, .. }
            | FsEvent::Modified { ts, .. }
            | FsEvent::Removed { ts, .. }
            | FsEvent::Renamed { ts, .. } => *ts,
            FsEvent::Batch(events) => events.last().map(|e| e.ts()).unwrap_or_else(now_secs),
        }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Helpers that don't need a full module.
pub fn monotonic_secs() -> f64 {
    let t = Instant::now();
    t.elapsed().as_secs_f64() // 0 right now, kept for symmetry
}

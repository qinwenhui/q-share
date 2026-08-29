//! On-demand FS watcher registry.
//!
//! The previous design spun up a single `RecursiveMode::Recursive`
//! `notify-debouncer-full` watcher on the entire share root at startup.
//! On a real-world root like `/Users/<name>` that walks every directory
//! (10⁵ entries easy) and never shuts down, which on macOS pushed the
//! process into GBs of resident memory and pinned a CPU core for minutes.
//!
//! This module replaces it with a [`WatcherRegistry`]: a `DashMap` of
//! per-directory watch handles, each created **only when a client
//! subscribes to a path** and reclaimed 30 s after the last unsubscribe.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::watcher::{now_secs, FsEvent};

/// How long a `WatchedDir` with zero subscribers may linger before the
/// janitor drops it. 30 s covers a quick "back / forward" navigation
/// without forcing a full re-subscribe round-trip.
pub const WATCH_TTL_SECS: u64 = 30;

/// One row in the registry.
pub struct WatchedDir {
    pub id: String,
    pub abs_path: PathBuf,
    pub recursive: bool,
    _debouncer: Arc<Mutex<Debouncer<notify::RecommendedWatcher, RecommendedCache>>>,
    pub tx: broadcast::Sender<FsEvent>,
    pub subs: Mutex<HashSet<String>>,
    pub last_active: Mutex<Instant>,
}

impl WatchedDir {
    pub fn new(
        abs_path: PathBuf,
        recursive: bool,
        sandbox_root: Arc<PathBuf>,
    ) -> Result<Self, notify::Error> {
        let (tx, _rx) = broadcast::channel::<FsEvent>(64);
        let id = Uuid::new_v4().to_string();
        let id_for_handler = id.clone();
        let tx_for_handler = tx.clone();
        let id_for_log = id.clone();
        let sandbox_root_clone = Arc::clone(&sandbox_root);

        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |res: DebounceEventResult| match res {
                Ok(events) => {
                    let mut batch: Vec<FsEvent> = Vec::with_capacity(events.len() * 2);
                    for ev in events {
                        for path in &ev.paths {
                            if let Some(out) = convert(&ev.kind, path, &sandbox_root_clone) {
                                batch.push(out);
                            }
                        }
                    }
                    if !batch.is_empty() {
                        let n = batch.len();
                        let _ = tx_for_handler.send(FsEvent::Batch(batch));
                        tracing::trace!(
                            watcher = %id_for_handler,
                            count = n,
                            "watcher batch dispatched"
                        );
                    }
                }
                Err(errs) => {
                    for e in errs {
                        tracing::warn!(watcher = %id_for_log, "watcher error: {e}");
                    }
                }
            },
        )?;
        let debouncer = Arc::new(Mutex::new(debouncer));
        let debouncer_for_init = Arc::clone(&debouncer);
        let abs_for_init = abs_path.clone();

        // Arm the watcher in a background thread. Initial recursive scan on
        // a large dir can take seconds; we don't block the caller. The
        // watcher is usable before this returns — only the very first
        // round of FS events for files already on disk are missed.
        let id_for_thread = id.clone();
        std::thread::Builder::new()
            .name(format!("qshare-watch-{}", &id[..8]))
            .spawn(move || {
                let mode = if recursive {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                let t = std::time::Instant::now();
                if let Err(e) = debouncer_for_init.lock().watch(&abs_for_init, mode) {
                    tracing::warn!(watcher = %id_for_thread, "watch arm failed: {e}");
                } else {
                    tracing::debug!(
                        watcher = %id_for_thread,
                        path = %abs_for_init.display(),
                        recursive,
                        elapsed_ms = t.elapsed().as_millis() as u64,
                        "watcher armed"
                    );
                }
            })
            .ok();

        Ok(Self {
            id,
            abs_path,
            recursive,
            _debouncer: debouncer,
            tx,
            subs: Mutex::new(HashSet::new()),
            last_active: Mutex::new(Instant::now()),
        })
    }

    pub fn subscriber_count(&self) -> usize {
        self.subs.lock().len()
    }

    pub fn touch(&self) {
        *self.last_active.lock() = Instant::now();
    }
}

#[derive(Clone)]
pub struct WatcherRegistry {
    inner: Arc<DashMap<PathBuf, Arc<WatchedDir>>>,
    sandbox_root: Arc<PathBuf>,
    pub log_tx: broadcast::Sender<WatcherLog>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatcherLog {
    Armed {
        path: String,
        recursive: bool,
        watcher_id: String,
        ts: u64,
    },
    Released {
        path: String,
        watcher_id: String,
        lifetime_secs: u64,
        ts: u64,
    },
}

impl WatcherRegistry {
    pub fn new(sandbox_root: Arc<PathBuf>) -> Self {
        let (log_tx, _rx) = broadcast::channel(256);
        Self {
            inner: Arc::new(DashMap::new()),
            sandbox_root,
            log_tx,
        }
    }

    /// Subscribe `conn_id` to `url_path`. Creates the watcher if missing.
    /// Returns a clone of the [`WatchedDir`] (cheap, Arc) plus a fresh
    /// receiver bound to that watcher's broadcast channel — the caller
    /// streams from this receiver and forwards events over WS.
    pub fn subscribe(
        &self,
        url_path: &str,
        recursive: bool,
        conn_id: &str,
    ) -> Result<(Arc<WatchedDir>, broadcast::Receiver<FsEvent>), crate::error::QshareError> {
        let abs = self.resolve(url_path)?;

        // Fast path: already exists.
        if let Some(w) = self.inner.get(&abs).map(|r| r.clone()) {
            w.subs.lock().insert(conn_id.to_string());
            w.touch();
            let rx = w.tx.subscribe();
            return Ok((w, rx));
        }

        // Cold path: create. DashMap's `entry().or_insert_with(...)` can't
        // return Err, so we do an explicit try-create-then-insert dance
        // to propagate watch arming failures.
        let watcher = match WatchedDir::new(abs.clone(), recursive, Arc::clone(&self.sandbox_root))
        {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!(path = %url_path, abs = %abs.display(), "watch arm failed: {e}");
                return Err(crate::error::QshareError::Internal(format!(
                    "watch arm failed for {url_path}: {e}"
                )));
            }
        };

        // Race-safe insert: another caller may have inserted while we
        // were arming. If so, drop ours and use theirs.
        let watcher = match self.inner.entry(abs.clone()) {
            dashmap::mapref::entry::Entry::Vacant(v) => {
                v.insert(watcher.clone());
                let _ = self.log_tx.send(WatcherLog::Armed {
                    path: url_path.to_string(),
                    recursive,
                    watcher_id: watcher.id.clone(),
                    ts: now_secs(),
                });
                tracing::info!(
                    path = %url_path,
                    abs = %abs.display(),
                    recursive,
                    id = %watcher.id,
                    "watcher armed"
                );
                watcher
            }
            dashmap::mapref::entry::Entry::Occupied(o) => {
                tracing::debug!(path = %url_path, "watcher race lost; using existing");
                o.get().clone()
            }
        };

        watcher.subs.lock().insert(conn_id.to_string());
        watcher.touch();
        let rx = watcher.tx.subscribe();
        Ok((watcher, rx))
    }

    /// Drop a subscription. Idempotent — calling for a connection that
    /// wasn't subscribed is a no-op.
    pub fn unsubscribe(&self, url_path: &str, conn_id: &str) {
        let Ok(abs) = self.resolve(url_path) else {
            return;
        };
        if let Some(w) = self.inner.get(&abs).map(|r| r.clone()) {
            w.subs.lock().remove(conn_id);
            w.touch();
        }
    }

    fn resolve(&self, url_path: &str) -> Result<PathBuf, crate::error::QshareError> {
        let trimmed = url_path.trim_start_matches('/');
        for comp in std::path::Path::new(trimmed).components() {
            use std::path::Component;
            if matches!(
                comp,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err(crate::error::QshareError::BadRequest(format!(
                    "invalid path: {url_path}"
                )));
            }
        }
        Ok(self.sandbox_root.join(trimmed))
    }

    pub fn sweep(&self) -> Vec<(PathBuf, String)> {
        let now = Instant::now();
        let to_drop: Vec<PathBuf> = self
            .inner
            .iter()
            .filter_map(|kv| {
                let w = kv.value();
                let last = *w.last_active.lock();
                let empty = w.subs.lock().is_empty();
                if empty && now.duration_since(last).as_secs() >= WATCH_TTL_SECS {
                    Some(kv.key().clone())
                } else {
                    None
                }
            })
            .collect();
        let mut released = Vec::new();
        for key in to_drop {
            if let Some((_, w)) = self.inner.remove(&key) {
                let lifetime = w.last_active.lock().elapsed().as_secs();
                let _ = self.log_tx.send(WatcherLog::Released {
                    path: w.abs_path.display().to_string(),
                    watcher_id: w.id.clone(),
                    lifetime_secs: lifetime,
                    ts: now_secs(),
                });
                tracing::info!(
                    path = %w.abs_path.display(),
                    id = %w.id,
                    "watcher released (no subs for {lifetime}s)"
                );
                released.push((key, w.id.clone()));
                // w drops here → debouncer drops → notify thread exits
            }
        }
        released
    }

    pub fn active_count(&self) -> usize {
        self.inner.len()
    }

    pub fn snapshot(&self) -> Vec<(String, String, usize)> {
        self.inner
            .iter()
            .map(|kv| {
                let w = kv.value();
                (
                    kv.key().display().to_string(),
                    w.id.clone(),
                    w.subscriber_count(),
                )
            })
            .collect()
    }
}

pub fn spawn_sweeper(registry: WatcherRegistry) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(15));
        tick.tick().await;
        loop {
            tick.tick().await;
            let _ = registry.sweep();
        }
    });
}

fn convert(kind: &notify::EventKind, abs: &Path, root: &Arc<PathBuf>) -> Option<FsEvent> {
    use notify::event::{ModifyKind, RenameMode};
    use notify::EventKind::*;

    let url = abs_to_url(abs, root)?;
    let is_dir = abs.is_dir()
        || (abs.extension().is_none() && abs.components().count() > 0 && !abs.exists());
    let ts = now_secs();

    Some(match kind {
        Create(_) => FsEvent::Created {
            path: PathBuf::from(url),
            is_dir,
            ts,
        },
        Modify(ModifyKind::Name(RenameMode::From)) => FsEvent::Removed {
            path: PathBuf::from(url),
            is_dir,
            ts,
        },
        Modify(ModifyKind::Name(RenameMode::To)) => FsEvent::Created {
            path: PathBuf::from(url),
            is_dir,
            ts,
        },
        Modify(_) => FsEvent::Modified {
            path: PathBuf::from(url),
            is_dir,
            ts,
        },
        Remove(_) => FsEvent::Removed {
            path: PathBuf::from(url),
            is_dir,
            ts,
        },
        _ => return None,
    })
}

fn abs_to_url(abs: &Path, root: &Path) -> Option<String> {
    let rel = abs.strip_prefix(root).ok()?;
    let mut out = String::new();
    for (i, comp) in rel.components().enumerate() {
        if i > 0 {
            out.push('/');
        }
        let s = comp.as_os_str().to_string_lossy();
        for byte in s.as_bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    out.push(*byte as char)
                }
                other => out.push_str(&format!("%{:02X}", other)),
            }
        }
    }
    Some(out)
}

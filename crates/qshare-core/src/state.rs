use std::path::PathBuf;
use std::sync::Arc;

use crate::cache::DirCache;
use crate::config::ServerConfig;
use crate::conn::{ConnectionRegistry, LogBuffer};
use crate::fs::sandbox::Sandbox;
use crate::stats::Stats;
use crate::thumbnail::ThumbnailCache;
use crate::watcher::WatcherRegistry;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub sandbox: Arc<Sandbox>,
    pub dir_cache: Arc<DirCache>,
    /// On-demand FS watcher — created lazily when a client subscribes to a
    /// path; reclaimed after 30 s of zero subscribers.
    pub watchers: WatcherRegistry,
    pub thumbs: Arc<ThumbnailCache>,
    /// Process-wide stats (active conns, bytes served, errors).
    pub stats: Arc<Stats>,
    /// Per-connection registry (id, ip, UA, path, bytes, uptime).
    pub connections: ConnectionRegistry,
    /// Rolling 500-line live log; GUI polls `/api/log` to render the panel.
    pub log: LogBuffer,
    /// Sandbox root, kept here so the WatcherRegistry can build absolute paths.
    pub sandbox_root: Arc<PathBuf>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> crate::Result<Self> {
        let sandbox = Sandbox::new(config.root.clone())?;
        let sandbox_root = Arc::new(sandbox.root().to_path_buf());
        let watchers = WatcherRegistry::new(Arc::clone(&sandbox_root));
        let thumbs = ThumbnailCache::new()
            .map_err(|e| crate::error::QshareError::Internal(format!("thumbnail cache: {e}")))?;
        Ok(Self {
            config: Arc::new(config),
            sandbox: Arc::new(sandbox),
            dir_cache: Arc::new(DirCache::new()),
            watchers,
            thumbs: Arc::new(thumbs),
            stats: Arc::new(Stats::new()),
            connections: ConnectionRegistry::new(),
            log: LogBuffer::new(),
            sandbox_root,
        })
    }

    /// Append a line to the live log.
    pub fn log_info(&self, level: &str, msg: impl Into<String>) {
        self.log.push(level, msg);
    }
}

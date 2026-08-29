use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;

/// On-disk thumbnail cache rooted at `~/.cache/q-share/thumbs/`.
pub struct ThumbnailCache {
    dir: PathBuf,
    /// Serializes concurrent first-generation of the same thumbnail, keyed by
    /// the same (path | mtime | width) disk-cache key. When several requests
    /// miss the cache at once (a browser fires grid + list + preview thumbs
    /// back-to-back), the first holds this per-key lock and decodes + resizes;
    /// the rest block, then re-check the disk cache and find it warm. Without
    /// it, each request would decode the full-size image independently.
    inflight: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl ThumbnailCache {
    pub fn new() -> Result<Self> {
        let dir = default_cache_dir()?;
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create thumbnail cache dir: {}", dir.display()))?;
        Ok(Self {
            dir,
            inflight: Mutex::new(HashMap::new()),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn put(&self, key: &str, bytes: &[u8]) -> Result<PathBuf> {
        let path = self.path_for(key);
        std::fs::write(&path, bytes)
            .with_context(|| format!("failed to write thumbnail: {}", path.display()))?;
        Ok(path)
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.path_for(key);
        std::fs::read(&path).ok()
    }

    /// Take the per-key "one generation at a time" lock. The first caller for
    /// a key gets it immediately; concurrent callers for the same key wait,
    /// then re-check [`Self::get`] for the peer's result.
    pub async fn inflight(&self, key: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = {
            let mut m = self.inflight.lock();
            m.entry(key.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }

    /// Forget a key once generation finished (success or failure), keeping the
    /// map bounded to thumbnails actually in flight. Safe to call while waiters
    /// still hold the key's lock — they hold clones of the same `Arc` and will
    /// still find the peer's cached result.
    pub fn release(&self, key: &str) {
        self.inflight.lock().remove(key);
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.jpg"))
    }
}

fn default_cache_dir() -> Result<PathBuf> {
    // XDG-style: $XDG_CACHE_HOME/q-share/thumbs or ~/.cache/q-share/thumbs.
    if let Ok(p) = std::env::var("QSHARE_CACHE_DIR") {
        return Ok(PathBuf::from(p).join("thumbs"));
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from)
        })
        .context("no HOME / USERPROFILE / XDG_CACHE_HOME set")?;
    Ok(base.join(".cache").join("q-share").join("thumbs"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn inflight_serializes_concurrent_same_key() {
        let cache = Arc::new(ThumbnailCache::new().unwrap());
        let g1 = cache.inflight("same-key").await;
        let c2 = Arc::clone(&cache);
        let waiter = tokio::spawn(async move {
            let _g2 = c2.inflight("same-key").await;
            true
        });
        // The second caller must block on the first's lock, not run parallel.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !waiter.is_finished(),
            "concurrent call for the same key must wait for the holder"
        );
        drop(g1);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), waiter)
                .await
                .is_ok(),
            "caller proceeds once the holder releases"
        );
    }

    #[tokio::test]
    async fn inflight_distinct_keys_do_not_block_each_other() {
        let cache = ThumbnailCache::new().unwrap();
        let _g1 = cache.inflight("key-a").await;
        // A different key gets its own lock immediately.
        let _g2 = cache.inflight("key-b").await;
        assert_eq!(cache.inflight.lock().len(), 2);
    }

    #[tokio::test]
    async fn release_forgets_key() {
        let cache = ThumbnailCache::new().unwrap();
        let g = cache.inflight("k").await;
        assert_eq!(cache.inflight.lock().len(), 1);
        drop(g);
        cache.release("k");
        assert_eq!(cache.inflight.lock().len(), 0);
    }
}

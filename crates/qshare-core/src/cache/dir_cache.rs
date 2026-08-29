use std::path::PathBuf;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use crate::fs::reader::DirListing;

/// Thread-safe TTL cache for directory listings keyed by absolute path.
pub struct DirCache {
    inner: RwLock<Inner>,
}

struct Inner {
    entries: std::collections::HashMap<PathBuf, CachedEntry>,
}

struct CachedEntry {
    listing: DirListing,
    created_at: Instant,
}

#[derive(Clone)]
pub struct DirListingSnapshot {
    pub listing: DirListing,
    pub age: Duration,
}

impl DirCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                entries: std::collections::HashMap::new(),
            }),
        }
    }

    pub fn get(&self, key: &PathBuf, ttl: Duration) -> Option<DirListingSnapshot> {
        let guard = self.inner.read();
        let entry = guard.entries.get(key)?;
        let age = entry.created_at.elapsed();
        if age > ttl {
            return None;
        }
        Some(DirListingSnapshot {
            listing: entry.listing.clone(),
            age,
        })
    }

    pub fn put(&self, key: PathBuf, listing: DirListing) {
        let mut guard = self.inner.write();
        guard.entries.insert(
            key,
            CachedEntry {
                listing,
                created_at: Instant::now(),
            },
        );
    }

    pub fn invalidate(&self, key: &PathBuf) {
        self.inner.write().entries.remove(key);
    }

    pub fn invalidate_prefix(&self, prefix: &PathBuf) {
        let mut guard = self.inner.write();
        guard.entries.retain(|k, _| !k.starts_with(prefix));
    }

    pub fn clear(&self) {
        self.inner.write().entries.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.read().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().entries.is_empty()
    }
}

impl Default for DirCache {
    fn default() -> Self {
        Self::new()
    }
}

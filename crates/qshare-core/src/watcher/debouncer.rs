// Legacy global watcher — kept only as a thin re-export shim so any
// leftover `use crate::watcher::debouncer::DebouncedWatcher` keeps
// compiling. The replacement lives in [`super::registry`] and is used
// per-directory on demand.

#[deprecated(note = "global recursive watcher removed; use crate::watcher::WatcherRegistry")]
pub struct DebouncedWatcher;

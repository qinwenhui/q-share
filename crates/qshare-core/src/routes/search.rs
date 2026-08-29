use std::path::Path;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::Json;
use ignore::{DirEntry as IgnoreDirEntry, WalkBuilder};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::fs::mime::preview_kind;
use crate::fs::reader::{read_sorted, DirEntry, SortKey, SortOrder};
use crate::state::AppState;

/// Defaults / limits for the search endpoint.
const DEFAULT_LIMIT: usize = 100;
const DEFAULT_MAX_RESULTS: usize = 500;
const DEFAULT_DEPTH: usize = 5;
const MAX_DEPTH: usize = 8;
const MAX_RESULTS_HARD_CAP: usize = 5_000;
/// Cap a single recursive walk at this many seconds. The walk is CPU-bound
/// (ignore::Walk synchronously walks the FS); bounding it keeps the request
/// from monopolising an axum worker on huge trees.
const RECURSIVE_TIMEOUT_SECS: u64 = 8;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub path: Option<String>,
    /// Legacy single-level `limit` — kept for back-compat. In recursive mode
    /// use `max_results` instead.
    #[serde(default)]
    pub limit: Option<usize>,
    /// When true, descend into subdirectories up to `depth` levels.
    #[serde(default)]
    pub recursive: bool,
    /// Max directory depth (default 5, hard-capped at 8).
    #[serde(default)]
    pub depth: Option<usize>,
    /// Cap on total returned hits (default 500, hard-capped at 5000).
    #[serde(default)]
    pub max_results: Option<usize>,
    /// Filter: `"file"` | `"dir"` | `"any"` (default any).
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub root: String,
    /// `false` for the legacy single-level mode (preserves old payload shape).
    pub recursive: bool,
    pub total: usize,
    /// True when the walk hit `max_results` (or the timeout) before exhausting
    /// the tree. Frontend should surface "showing first N results — refine
    /// your search to see more".
    pub truncated: bool,
    pub results: Vec<SearchHit>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub entry: DirEntry,
    /// Path of the hit relative to the searched root (e.g. `"summer/photo.jpg"`).
    pub rel_path: String,
    /// Absolute URL path of the directory containing this hit. Useful for the
    /// frontend to group results "by directory".
    pub parent_dir: String,
    /// 0 for direct children, 1+ for nested hits.
    pub depth: usize,
}

pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    let needle = q.q.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Json(SearchResponse {
            query: String::new(),
            root: String::new(),
            recursive: false,
            total: 0,
            truncated: false,
            results: Vec::new(),
        }));
    }
    let url_path = q.path.unwrap_or_default();
    let url_path = if url_path.is_empty() {
        "/".into()
    } else {
        url_path
    };
    let abs = state.sandbox.resolve(&url_path)?;
    let kind_filter = parse_kind_filter(q.kind.as_deref());

    if !q.recursive {
        let mut resp = shallow_search(
            &state,
            &url_path,
            &abs,
            &needle,
            q.limit.unwrap_or(DEFAULT_LIMIT),
            kind_filter,
        )
        .await?;
        resp.query = q.q.clone();
        return Ok(Json(resp));
    }

    let depth = q.depth.unwrap_or(DEFAULT_DEPTH).clamp(1, MAX_DEPTH);
    let max_results = q
        .max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .clamp(1, MAX_RESULTS_HARD_CAP);
    // ignore::WalkBuilder::max_depth counts the root itself, so user-facing
    // "depth N" (show up to N nested levels) maps to N+1 for the walker.
    let walker_max_depth = depth + 1;

    let hits = recursive_search(
        &state,
        &url_path,
        &abs,
        &needle,
        walker_max_depth,
        max_results,
        kind_filter,
    )
    .await?;

    let truncated = hits.len() >= max_results;
    let total = hits.len();

    Ok(Json(SearchResponse {
        query: q.q,
        root: url_path,
        recursive: true,
        total,
        truncated,
        results: hits,
    }))
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum KindFilter {
    Any,
    File,
    Dir,
}

fn parse_kind_filter(s: Option<&str>) -> KindFilter {
    match s.map(|s| s.to_ascii_lowercase()) {
        Some(k) if k == "file" => KindFilter::File,
        Some(k) if k == "dir" || k == "directory" => KindFilter::Dir,
        _ => KindFilter::Any,
    }
}

/// Legacy single-level path. Re-uses `read_sorted` + the dir cache so it stays
/// fast for the common case ("fuzzy-search the folder I'm looking at").
async fn shallow_search(
    state: &AppState,
    url_path: &str,
    abs: &Path,
    needle: &str,
    limit: usize,
    kind: KindFilter,
) -> Result<SearchResponse> {
    let ttl = Duration::from_secs(state.config.cache_ttl_secs.max(1));
    let key = abs.to_path_buf();

    let entries = if let Some(snap) = state.dir_cache.get(&key, ttl) {
        snap.listing.entries
    } else {
        let entries = read_sorted(
            abs,
            url_path,
            SortKey::Name,
            SortOrder::Asc,
            state.config.show_hidden,
        )
        .await?;
        state.dir_cache.put(
            key,
            crate::fs::reader::DirListing {
                path: url_path.to_string(),
                parent: None,
                entries: entries.clone(),
                total: entries.len(),
            },
        );
        entries
    };

    let mut filtered: Vec<SearchHit> = entries
        .into_iter()
        .filter(|e| matches_kind(e, kind))
        .filter(|e| e.name.to_lowercase().contains(needle))
        .map(|entry| {
            let rel_path = entry.name.clone();
            let parent_dir = url_path.to_string();
            SearchHit {
                entry,
                rel_path,
                parent_dir,
                depth: 0,
            }
        })
        .collect();
    filtered.truncate(limit);

    Ok(SearchResponse {
        query: String::new(), // caller fills
        root: url_path.to_string(),
        recursive: false,
        total: filtered.len(),
        truncated: false,
        results: filtered,
    })
}

/// Bounded recursive search. Runs `ignore::Walk` on a blocking thread because
/// it does synchronous filesystem syscalls. Returns up to `max_results` hits.
///
/// `ignore::WalkBuilder` gives us .gitignore / .git / node_modules skipping
/// for free, which is what we want — searching a project's node_modules is
/// useless and slow.
async fn recursive_search(
    state: &AppState,
    url_root: &str,
    abs_root: &Path,
    needle: &str,
    walker_max_depth: usize,
    max_results: usize,
    kind: KindFilter,
) -> Result<Vec<SearchHit>> {
    let sandbox_root = state.sandbox.root().to_path_buf();
    let abs_root_owned = abs_root.to_path_buf();
    let url_root_owned = url_root.to_string();
    let needle_owned = needle.to_string();

    let (tx, rx) = tokio::sync::mpsc::channel::<SearchHit>(max_results + 64);

    // Run the synchronous walker on a blocking thread. CPU-bound work
    // doesn't belong on the async runtime's worker threads.
    let _walker = tokio::task::spawn_blocking(move || {
        let mut builder = WalkBuilder::new(&abs_root_owned);
        builder
            .follow_links(false)
            .standard_filters(true) // honours .gitignore, .git, .hg, …
            .max_depth(Some(walker_max_depth))
            .threads(num_cpus_for_walk())
            .require_git(false); // don't error if not in a git repo
                                 // `standard_filters` honours .gitignore but does NOT skip node_modules
                                 // / .cache / target etc. by themselves — write a temp gitignore-style
                                 // file and feed it through `WalkBuilder::add_ignore` so the walker
                                 // skips these toolchain noise directories at every level.
        let noise_ignore = b"\
node_modules/
.cache/
target/
dist/
.next/
.nuxt/
.venv/
__pycache__/
.DS_Store
build/
";
        if let Some(tmp) = std::env::temp_dir().join("qshare-noise.ignore").to_str() {
            let _ = std::fs::write(tmp, noise_ignore);
            let _ = builder.add_ignore(tmp);
        }

        let mut pushed = 0usize;
        let mut truncated = false;

        for dent in builder.build().flatten() {
            if pushed >= max_results {
                truncated = true;
                break;
            }
            if let Some(hit) = walk_hit(&sandbox_root, &url_root_owned, &dent, &needle_owned, kind)
            {
                // Channel may be closed if the consumer hit max_results early
                // and dropped — that's fine, just stop walking.
                if tx.blocking_send(hit).is_err() {
                    break;
                }
                pushed += 1;
            }
        }
        truncated
    });

    // Drain on a deadline so a huge tree can't tie up the worker forever.
    let deadline = tokio::time::Duration::from_secs(RECURSIVE_TIMEOUT_SECS);
    let mut hits: Vec<SearchHit> = Vec::with_capacity(max_results);
    let collect_fut = async {
        let mut rx = rx;
        while let Some(hit) = rx.recv().await {
            hits.push(hit);
            if hits.len() >= max_results {
                break;
            }
        }
    };
    let _ = tokio::time::timeout(deadline, collect_fut).await;
    // The walker task will finish when it exhausts the tree or hits its own
    // limit. Dropping the channel signals it to stop early on the consumer
    // side (max_results reached or deadline exceeded). We don't `await` it —
    // the request can return as soon as we have enough hits, and the walker
    // will exit in the background.

    Ok(hits)
}

fn walk_hit(
    sandbox_root: &Path,
    url_root: &str,
    dent: &IgnoreDirEntry,
    needle: &str,
    kind: KindFilter,
) -> Option<SearchHit> {
    if !dent
        .file_type()
        .map(|ft| ft.is_dir() || ft.is_file())
        .unwrap_or(false)
    {
        return None;
    }
    let path = dent.path();
    let file_name = path.file_name()?.to_str()?;
    if !file_name.to_lowercase().contains(needle) {
        return None;
    }
    let is_dir = dent.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
    if !matches_kind_predicate(is_dir, kind) {
        return None;
    }

    // Compute the URL path relative to the sandbox root, so it's stable even
    // when the absolute prefix on disk contains weird characters.
    let rel_to_root = path.strip_prefix(sandbox_root).unwrap_or(path);
    let url_path = abs_to_url_path(rel_to_root);

    // The hit lives inside the searched root — strip that prefix to get
    // a relative display path. If the walker's root is the root of the
    // share, the result starts with "/", which we trim.
    let parent_abs = path.parent()?;
    let parent_url = if parent_abs == sandbox_root {
        "/".to_string()
    } else {
        let parent_rel = parent_abs.strip_prefix(sandbox_root).unwrap_or(parent_abs);
        abs_to_url_path(parent_rel)
    };
    let _ = url_root; // (kept for symmetry / future filtering on root)

    // Skip the root itself if it somehow matches.
    if rel_to_root.as_os_str().is_empty() {
        return None;
    }

    let depth = rel_to_root.components().count().saturating_sub(1);

    let entry = build_entry(rel_to_root, &url_path, is_dir);
    let rel_path = url_path.trim_start_matches('/').to_string();

    Some(SearchHit {
        entry,
        rel_path,
        parent_dir: parent_url,
        depth,
    })
}

fn matches_kind(entry: &DirEntry, kind: KindFilter) -> bool {
    match kind {
        KindFilter::Any => true,
        KindFilter::File => !entry.is_dir,
        KindFilter::Dir => entry.is_dir,
    }
}

fn matches_kind_predicate(is_dir: bool, kind: KindFilter) -> bool {
    match kind {
        KindFilter::Any => true,
        KindFilter::File => !is_dir,
        KindFilter::Dir => is_dir,
    }
}

fn build_entry(abs: &Path, url_path: &str, is_dir: bool) -> DirEntry {
    let name = abs
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mime = if is_dir {
        None
    } else {
        Some(crate::fs::reader::mime_for(&name))
    };
    let previewable = !is_dir
        && mime
            .as_deref()
            .map(|m| preview_kind(m).is_previewable())
            .unwrap_or(false);

    let meta = std::fs::symlink_metadata(abs).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    DirEntry {
        name,
        path: url_path.to_string(),
        is_dir,
        size,
        modified,
        mime,
        previewable,
    }
}

fn abs_to_url_path(p: &Path) -> String {
    let mut out = String::from("/");
    for (i, seg) in p.components().enumerate() {
        let s = match seg {
            std::path::Component::Normal(s) => s.to_string_lossy().into_owned(),
            _ => continue,
        };
        if i > 0 {
            out.push('/');
        }
        out.push_str(&utf8_percent_encode(&s, NON_ALPHANUMERIC).to_string());
    }
    if out.len() > 1 {
        out
    } else {
        "/".to_string()
    }
}

fn num_cpus_for_walk() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().clamp(1, 4))
        .unwrap_or(2)
}

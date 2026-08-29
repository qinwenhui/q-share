use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::{QshareError, Result};
use crate::fs::mime::preview_kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
    Modified,
}

impl SortKey {
    pub fn parse(s: &str) -> Self {
        match s {
            "size" => SortKey::Size,
            "modified" | "mtime" => SortKey::Modified,
            _ => SortKey::Name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "desc" | "descending" => SortOrder::Desc,
            _ => SortOrder::Asc,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String, // URL path relative to shared root
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    pub mime: Option<String>,
    /// Whether the entry can be rendered inline by the frontend
    /// (image / video / audio / pdf / text). Lets the grid view
    /// decide whether to show a thumbnail or a generic icon.
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
    pub total: usize,
}

/// Read a directory, sort it, and return a paginated listing.
pub async fn read_dir_listing(
    abs_dir: &Path,
    url_path: &str,
    sort: SortKey,
    order: SortOrder,
    offset: usize,
    limit: usize,
    show_hidden: bool,
) -> Result<DirListing> {
    if !abs_dir.is_dir() {
        return Err(QshareError::NotADirectory(url_path.to_string()));
    }

    let mut entries = read_sorted(abs_dir, url_path, sort, order, show_hidden).await?;
    let total = entries.len();
    let page: Vec<DirEntry> = entries.drain(offset..(offset + limit).min(total)).collect();

    let parent = compute_parent(url_path);

    Ok(DirListing {
        path: url_path.to_string(),
        parent,
        entries: page,
        total,
    })
}

/// Read all entries in `dir`, filtering and sorting. No pagination.
pub async fn read_sorted(
    dir: &Path,
    url_path: &str,
    sort: SortKey,
    order: SortOrder,
    show_hidden: bool,
) -> Result<Vec<DirEntry>> {
    let mut rd = tokio::fs::read_dir(dir).await?;
    let mut out = Vec::new();

    while let Some(entry) = rd.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        let ft = match entry.file_type().await {
            Ok(t) => t,
            Err(_) => continue,
        };
        let meta = entry.metadata().await.ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let child_url = join_url_path(url_path, &name);
        let mime = if ft.is_dir() {
            None
        } else {
            Some(mime_for(&name))
        };
        // Dirs can't be previewed inline; for files, ask the MIME layer.
        let previewable = !ft.is_dir()
            && mime
                .as_deref()
                .map(|m| preview_kind(m).is_previewable())
                .unwrap_or(false);

        out.push(DirEntry {
            name,
            path: child_url,
            is_dir: ft.is_dir(),
            size,
            modified,
            mime,
            previewable,
        });
    }

    // Directories first, then by sort key.
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir) // dirs first
            .then_with(|| cmp_key(a, b, sort, order))
    });

    Ok(out)
}

fn cmp_key(a: &DirEntry, b: &DirEntry, sort: SortKey, order: SortOrder) -> std::cmp::Ordering {
    let ord = match sort {
        SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::Modified => a.modified.cmp(&b.modified),
    };
    match order {
        SortOrder::Asc => ord,
        SortOrder::Desc => ord.reverse(),
    }
}

fn join_url_path(base: &str, name: &str) -> String {
    if base.is_empty() || base == "/" {
        format!("/{}", encode(name))
    } else {
        format!("{}/{}", base.trim_end_matches('/'), encode(name))
    }
}

fn compute_parent(url_path: &str) -> Option<String> {
    let trimmed = url_path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return None;
    }
    let idx = trimmed.rfind('/')?;
    if idx == 0 {
        Some("/".to_string())
    } else {
        Some(trimmed[..idx].to_string())
    }
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}

/// Quick filename-based MIME guess wrapper.
pub fn mime_for(name: &str) -> String {
    mime_guess::from_path(name)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_of_root_is_none() {
        assert_eq!(compute_parent("/"), None);
        assert_eq!(compute_parent(""), None);
    }

    #[test]
    fn parent_of_first_level() {
        assert_eq!(compute_parent("/foo"), Some("/".into()));
    }

    #[test]
    fn parent_of_nested() {
        assert_eq!(compute_parent("/foo/bar/baz"), Some("/foo/bar".into()));
    }

    #[test]
    fn join_handles_empty_base() {
        assert_eq!(join_url_path("", "a"), "/a");
        assert_eq!(join_url_path("/", "a"), "/a");
    }

    #[test]
    fn join_handles_existing_base() {
        assert_eq!(join_url_path("/foo", "bar"), "/foo/bar");
        assert_eq!(join_url_path("/foo/", "bar"), "/foo/bar");
    }
}

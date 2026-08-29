use std::path::Path;
use std::time::UNIX_EPOCH;

use axum::extract::{Query, State};
use axum::Json;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::error::Result;
use crate::fs::mime::preview_kind;
use crate::fs::reader::mime_for;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct StatQuery {
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct StatResponse {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    /// Unix mode bits as a 4-char string (e.g. `"0644"`). Empty for Windows
    /// / unsupported platforms.
    pub mode: String,
    pub mime: Option<String>,
    pub etag: String,
    pub previewable: bool,
    /// Hex MD5 of the file's bytes. `None` for directories (they have no
    /// hashable content) and for files larger than [`MD5_MAX_BYTES`] —
    /// computing a multi-GB MD5 synchronously in a request handler is a
    /// bad idea, and the UI shouldn't pretend to know the answer.
    pub md5: Option<String>,
}

/// Files smaller than this have their MD5 computed inline as part of the
/// stat response. Anything bigger gets `md5: null` and the UI falls back
/// to a separate `/api/hash` endpoint for on-demand hashing.
const MD5_MAX_BYTES: u64 = 64 * 1024 * 1024;

pub async fn stat(
    State(state): State<AppState>,
    Query(q): Query<StatQuery>,
) -> Result<Json<StatResponse>> {
    let abs = state.sandbox.resolve(&q.path)?;
    let meta = tokio::fs::metadata(&abs).await?;
    let name = abs
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mime = if meta.is_dir() {
        None
    } else {
        Some(mime_for(&name))
    };
    let etag = format!("\"{}-{}\"", size, modified);
    let previewable = !meta.is_dir()
        && mime
            .as_deref()
            .map(|m| preview_kind(m).is_previewable())
            .unwrap_or(false);

    let mode = mode_string(&abs);

    // MD5 inline for small files only — large files would block the worker.
    let md5 = if !meta.is_dir() && size > 0 && size <= MD5_MAX_BYTES {
        Some(compute_md5(&abs).await?)
    } else {
        None
    };

    Ok(Json(StatResponse {
        path: q.path,
        name,
        is_dir: meta.is_dir(),
        size,
        modified,
        mode,
        mime,
        etag,
        previewable,
        md5,
    }))
}

/// Render Unix mode bits as a 4-digit octal string ("0644"). Returns "" if
/// the platform doesn't expose the bits (Windows / FAT / sandboxed mounts).
#[cfg(unix)]
fn mode_string(path: &Path) -> String {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| format!("{:04o}", m.permissions().mode() & 0o7777))
        .unwrap_or_default()
}

#[cfg(not(unix))]
fn mode_string(_path: &Path) -> String {
    String::new()
}

/// Stream the file through an MD5 hasher. Reads in 64 KB chunks so we
/// don't pin a multi-MB file in memory all at once.
async fn compute_md5(path: &Path) -> std::io::Result<String> {
    let mut f = tokio::fs::File::open(path).await?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

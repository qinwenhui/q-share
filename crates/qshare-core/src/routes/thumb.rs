//! Thumbnail endpoint: `GET /api/thumb?path=&w=`.
//!
//! Resolves the path safely, generates a JPEG thumbnail on first request,
//! caches to disk by (path, mtime, requested width) and returns the cached
//! bytes on subsequent requests. 1-day TTL on cached entries is enforced
//! loosely via mtime checks (we re-key by file mtime so edits invalidate).

use std::time::UNIX_EPOCH;

use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::error::QshareError;
use crate::state::AppState;
use crate::thumbnail::{cache_key, generate_thumbnail, ThumbResult};

#[derive(Debug, Deserialize)]
pub struct ThumbQuery {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub w: Option<u32>,
}

const MAX_WIDTH: u32 = 1024;

pub async fn thumb(State(state): State<AppState>, Query(q): Query<ThumbQuery>) -> Response {
    let abs = match state.sandbox.resolve(&q.path) {
        Ok(p) => p,
        Err(e) => return e.into_response(),
    };
    let max_w = q.w.unwrap_or(320).clamp(16, MAX_WIDTH);

    let meta = match tokio::fs::metadata(&abs).await {
        Ok(m) => m,
        Err(e) => return QshareError::Io(e).into_response(),
    };
    if meta.is_dir() {
        return (StatusCode::BAD_REQUEST, "thumb of directory").into_response();
    }
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let key = cache_key(&abs, modified, max_w);

    if let Some(bytes) = state.thumbs.get(&key) {
        return jpeg_response(bytes, true);
    }

    // Cold miss: several requests can arrive for the same (path, mtime, width)
    // before anyone writes the disk cache — a browser fires grid + list +
    // preview thumbs back-to-back. Hold the per-key lock so exactly one caller
    // decodes + resizes; the rest wait, then hit the warm cache below.
    let _guard = state.thumbs.inflight(&key).await;

    if let Some(bytes) = state.thumbs.get(&key) {
        // A peer finished generating while we waited for the lock.
        return jpeg_response(bytes, true);
    }

    let bytes = match generate_thumbnail(&abs, max_w) {
        Ok(ThumbResult::Jpeg(b)) => b,
        Ok(ThumbResult::Unsupported(reason)) => {
            state.thumbs.release(&key);
            return (StatusCode::UNSUPPORTED_MEDIA_TYPE, reason).into_response();
        }
        Err(e) => {
            tracing::debug!("thumbnail failed for {}: {e}", abs.display());
            state.thumbs.release(&key);
            return (StatusCode::UNPROCESSABLE_ENTITY, e).into_response();
        }
    };

    if let Err(e) = state.thumbs.put(&key, &bytes) {
        tracing::warn!("thumbnail cache write failed: {e}");
    }
    state.thumbs.release(&key);
    jpeg_response(bytes, false)
}

fn jpeg_response(bytes: Vec<u8>, from_cache: bool) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    headers.insert(
        header::HeaderName::from_static("x-thumbnail-cache"),
        HeaderValue::from_static(if from_cache { "hit" } else { "miss" }),
    );
    (StatusCode::OK, headers, bytes).into_response()
}

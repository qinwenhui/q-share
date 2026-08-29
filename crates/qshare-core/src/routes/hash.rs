//! On-demand file hashing — used by the "Details" modal in the web UI
//! when the file is too large to fit in the inline MD5 on `/api/stat`.
//!
//! Streams the file in 64 KB chunks so we don't pin a multi-GB file in
//! memory. Returns the hex digest as plain text (the most common case
//! is MD5, but the query parameter `algo` lets us ask for sha256 too).

use std::path::Path;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use md5::{Digest, Md5};
use serde::Deserialize;
use sha2::Sha256;
use tokio::io::AsyncReadExt;

use crate::error::Result;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct HashQuery {
    #[serde(default)]
    pub path: String,
    /// `md5` (default) or `sha256`.
    #[serde(default)]
    pub algo: Option<String>,
}

pub async fn hash(State(state): State<AppState>, Query(q): Query<HashQuery>) -> Result<Response> {
    let abs = state.sandbox.resolve(&q.path)?;
    let meta = tokio::fs::metadata(&abs).await?;
    if meta.is_dir() {
        // Directories don't have a content hash — return an explicit
        // error string so the frontend can render it.
        return Ok(error_response("directories have no content hash"));
    }

    // Cap a single hash run so a hostile 50 GB file can't pin the worker
    // forever. 60 s is plenty for several GB on SSD; bigger files should
    // use a real offline tool.
    let fut = compute_hash(&abs, q.algo.as_deref().unwrap_or("md5"));
    let digest = tokio::time::timeout(Duration::from_secs(60), fut)
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "hash timed out"))??;

    Ok(plain_text_response(&digest))
}

async fn compute_hash(path: &Path, algo: &str) -> std::io::Result<String> {
    let mut f = tokio::fs::File::open(path).await?;
    let mut buf = vec![0u8; 64 * 1024];
    match algo {
        "sha256" => {
            let mut h = Sha256::new();
            loop {
                let n = f.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(format!("{:x}", h.finalize()))
        }
        // md5 is the default — it's what most users mean by "hash a file".
        _ => {
            let mut h = Md5::new();
            loop {
                let n = f.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(format!("{:x}", h.finalize()))
        }
    }
}

fn plain_text_response(s: &str) -> Response {
    let body = s.as_bytes().to_vec();
    let mut resp = body.into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    resp
}

fn error_response(msg: &str) -> Response {
    use axum::http::StatusCode;
    (
        StatusCode::BAD_REQUEST,
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        msg.to_string(),
    )
        .into_response()
}

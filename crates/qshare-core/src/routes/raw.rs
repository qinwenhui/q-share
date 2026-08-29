use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::error::{QshareError, Result};
use crate::range::RangeSpec;
use crate::state::AppState;

const HEADER_IF_NONE_MATCH: &str = "if-none-match";
const HEADER_ACCEPT_RANGES: &str = "accept-ranges";

#[derive(Debug, Deserialize)]
pub struct RawQuery {
    #[serde(default)]
    pub path: String,
}

pub async fn raw(
    State(state): State<AppState>,
    headers_in: HeaderMap,
    Query(q): Query<RawQuery>,
) -> Result<Response> {
    let abs = state.sandbox.resolve(&q.path)?;
    let meta = tokio::fs::metadata(&abs).await?;
    if meta.is_dir() {
        return Err(QshareError::BadRequest("cannot stream a directory".into()));
    }
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let etag = format!("\"{}-{}\"", size, modified);
    let mime = mime_guess::from_path(&abs).first_or_octet_stream();

    if let Some(inm) = headers_in
        .get(HEADER_IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    {
        if inm == etag {
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
            resp_headers.insert(HEADER_ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            return Ok((StatusCode::NOT_MODIFIED, resp_headers).into_response());
        }
    }

    let range_header = headers_in.get(header::RANGE).and_then(|v| v.to_str().ok());

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.essence_str()).unwrap(),
    );
    resp_headers.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    resp_headers.insert(HEADER_ACCEPT_RANGES, HeaderValue::from_static("bytes"));

    let (status, start, length) = match range_header.and_then(|h| RangeSpec::parse(h, size)) {
        Some(r) => (StatusCode::PARTIAL_CONTENT, r.start, r.length),
        None => (StatusCode::OK, 0, size),
    };

    if let Some(r) = range_header.and_then(|h| RangeSpec::parse(h, size)) {
        let cr = format!("bytes {}-{}/{}", r.start, r.start + r.length - 1, size);
        if let Ok(v) = HeaderValue::from_str(&cr) {
            resp_headers.insert(header::CONTENT_RANGE, v);
        }
    }

    let stream = file_range_stream(abs, start, length);
    let body = Body::from_stream(stream);
    resp_headers.insert(header::CONTENT_LENGTH, HeaderValue::from(length));
    Ok((status, resp_headers, body).into_response())
}

fn file_range_stream(
    path: std::path::PathBuf,
    start: u64,
    length: u64,
) -> impl futures::Stream<Item = std::io::Result<Bytes>> + Send + 'static {
    async_stream::stream! {
        let mut f = tokio::fs::File::open(&path).await?;
        f.seek(std::io::SeekFrom::Start(start)).await?;
        let mut limited = f.take(length);
        // 16 KB chunks — small enough that client cancellations propagate
        // quickly through the TCP send buffer, so the byte counter doesn't
        // inflate on every video scrub.
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            let n = limited.read(&mut buf).await?;
            if n == 0 { break; }
            yield Ok(Bytes::copy_from_slice(&buf[..n]));
        }
    }
}

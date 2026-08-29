use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, QshareError>;

#[derive(Debug, Error)]
pub enum QshareError {
    #[error("path not found: {0}")]
    NotFound(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("not a directory: {0}")]
    NotADirectory(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrBody<'a> {
    error: &'a str,
    message: String,
}

impl IntoResponse for QshareError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            QshareError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            QshareError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            QshareError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            QshareError::NotADirectory(_) => (StatusCode::BAD_REQUEST, "not_a_directory"),
            QshareError::Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, "not_found"),
                std::io::ErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, "forbidden"),
                _ => (StatusCode::INTERNAL_SERVER_ERROR, "io_error"),
            },
            QshareError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        let body = ErrBody {
            error: code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

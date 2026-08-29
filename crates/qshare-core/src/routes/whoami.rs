use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::Json;
use serde::Serialize;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct WhoamiResponse {
    pub version: &'static str,
    pub hostname: String,
    pub client_ip: String,
    pub shared_root: String,
    pub url: String,
}

pub async fn whoami(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Json<WhoamiResponse> {
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "q-share".into());

    Json(WhoamiResponse {
        version: env!("CARGO_PKG_VERSION"),
        hostname,
        client_ip: addr.ip().to_string(),
        shared_root: state.sandbox.root().display().to_string(),
        url: state.config.url(),
    })
}

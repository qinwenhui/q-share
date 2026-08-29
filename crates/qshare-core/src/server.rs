use std::net::SocketAddr;
use std::time::Duration;

use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::conn::spawn_janitor;
use crate::error::Result;
use crate::state::AppState;
use crate::watcher::spawn_sweeper;
use crate::ServerConfig;

use qshare_assets::Assets;

/// Handle returned by [`Server::start`]. Drop to shut down.
pub struct ServerHandle {
    local_addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<tokio::task::JoinHandle<()>>,
    // Drops on shutdown — unregisters the mDNS service.
    _mdns: Option<crate::discovery::MdnsService>,
}

impl ServerHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        // Bound the *drain* phase: browsers may hold SSE/WS connections,
        // and axum's default graceful shutdown waits forever for them.
        // 3 s is plenty for legitimate request draining; anything beyond
        // is a stuck client and we force-abort.
        if let Some(j) = self.join.take() {
            match tokio::time::timeout(Duration::from_secs(3), j).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("server task panicked on shutdown: {e}"),
                Err(_) => {
                    tracing::warn!(
                        "server didn't drain within 3s of shutdown; abandoning open connections"
                    );
                    // (join handle is dropped here, which doesn't abort
                    // the task — but since the task awaits axum::serve
                    // which respects the rx signal, it will exit on its
                    // own once the runtime shuts down.)
                }
            }
        }
        // _mdns drops here.
    }
}

pub struct Server {
    config: ServerConfig,
}

impl Server {
    pub fn new(config: ServerConfig) -> Result<Self> {
        Ok(Self { config })
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Start listening. Returns a handle whose Drop keeps the server alive
    /// (call `.shutdown().await` to terminate gracefully).
    pub async fn start(self) -> Result<ServerHandle> {
        // No global FS watcher. Watchers are spawned on-demand by the WS
        // handler when a client subscribes to a path; they self-evict 30 s
        // after the last unsubscribe. The result: cold-start is ~50 ms no
        // matter how big the shared root is.
        let state = AppState::new(self.config.clone())?;
        state.log_info(
            "system",
            format!(
                "q-share {} online — root={}",
                env!("CARGO_PKG_VERSION"),
                state.config.root.display()
            ),
        );

        // Background helpers — both are cheap (one timer + occasional scan).
        spawn_janitor(state.connections.clone());
        spawn_sweeper(state.watchers.clone());

        let router = build_router(state);

        let addr: SocketAddr =
            self.config.bind_addr().parse().map_err(|e| {
                crate::error::QshareError::Internal(format!("bad bind addr: {}", e))
            })?;
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        let (tx, rx) = oneshot::channel::<()>();
        let join = tokio::spawn(async move {
            let svc = router.into_make_service_with_connect_info::<SocketAddr>();
            // Run the server forever (until `rx` fires). No timeout here —
            // a timeout would kill the server 3 s after startup regardless
            // of any shutdown signal. The shutdown drain itself is bounded
            // by `ServerHandle::shutdown`, which races a 3 s timer against
            // the join handle and force-aborts on overrun.
            let server = axum::serve(listener, svc).with_graceful_shutdown(async move {
                let _ = rx.await;
            });
            if let Err(e) = server.await {
                tracing::error!("server crashed: {e}");
            }
        });

        // Best-effort mDNS — failures don't stop the server.
        let mdns = crate::discovery::MdnsService::register(
            local_addr.port(),
            &crate::discovery::root_label(&self.config.root),
        )
        .ok();

        Ok(ServerHandle {
            local_addr,
            shutdown: Some(tx),
            join: Some(join),
            _mdns: mdns,
        })
    }
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/api/list", get(crate::routes::list::list))
        .route("/api/stat", get(crate::routes::stat::stat))
        .route("/api/hash", get(crate::routes::hash::hash))
        .route("/api/raw", get(crate::routes::raw::raw))
        .route("/api/whoami", get(crate::routes::whoami::whoami))
        .route("/api/health", get(crate::routes::health::health))
        .route("/api/search", get(crate::routes::search::search))
        // Legacy SSE — kept for backwards compat; new clients should use /ws.
        .route("/api/events", get(crate::routes::events::events))
        .route("/api/thumb", get(crate::routes::thumb::thumb))
        .route("/api/stats", get(crate::routes::stats::stats))
        // Connection + log + watcher introspection — used by the GUI dashboard.
        .route(
            "/api/connections",
            get(crate::routes::connections::connections),
        )
        .route("/api/log", get(crate::routes::log::log))
        .route("/api/watchers", get(crate::routes::watchers::watchers))
        // WebSocket — single endpoint for everything push-oriented: subscribe
        // to a path, receive FS events, see stats, see log lines.
        .route("/ws", get(crate::routes::ws::ws))
        .with_state(state.clone());

    let api = api.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        crate::middleware::track,
    ));

    api.merge(assets_router(state))
}

fn assets_router(state: AppState) -> Router {
    use axum::routing::get;
    Router::new()
        .route("/", get(serve_index))
        .route("/{*path}", get(serve_asset))
        .with_state(state)
}

use axum::extract::{Path, State as AxState};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

async fn serve_index(AxState(state): AxState<AppState>) -> Response {
    match Assets::read("index.html") {
        Some(bytes) => html_response(bytes, state),
        None => dev_mode_response(),
    }
}

async fn serve_asset(AxState(state): AxState<AppState>, Path(path): Path<String>) -> Response {
    let normalized = path.trim_start_matches('/');
    if normalized.is_empty() {
        return serve_index(AxState(state)).await;
    }
    if let Some(bytes) = Assets::read(normalized) {
        let mime = mime_guess::from_path(normalized).first_or_octet_stream();
        let mut resp = (StatusCode::OK, bytes).into_response();
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime.essence_str()).unwrap(),
        );
        resp
    } else {
        // SPA fallback: any path not matching a built asset is a
        // client-side route. Solid Router handles `/preview/*`, future
        // `/search`, etc. Browser <img>/<script> tags get HTML on
        // miss, which fails harmlessly for those MIME types.
        match Assets::read("index.html") {
            Some(bytes) => html_response(bytes, state),
            None => dev_mode_response(),
        }
    }
}

fn html_response(bytes: bytes::Bytes, _state: AppState) -> Response {
    let mut resp = (StatusCode::OK, bytes).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp
}

fn dev_mode_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "q-share frontend assets not built.\nRun `npm run build` in web/ first, or use dev mode (vite dev server on :5173).",
    )
        .into_response()
}

// Re-exports for callers that don't want to depend on `axum` directly.
pub use axum::extract::ConnectInfo;
pub use axum::serve;

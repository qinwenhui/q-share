//! qshare-core: file sharing server core (HTTP + FS + cache)
//!
//! Provides the [`Server`] builder used by all UI frontends.

pub mod cache;
pub mod config;
pub mod conn;
pub mod discovery;
pub mod error;
pub mod fs;
pub mod middleware;
pub mod range;
pub mod routes;
pub mod server;
pub mod state;
pub mod stats;
pub mod thumbnail;
pub mod watcher;

pub use config::ServerConfig;
pub use discovery::{root_label, MdnsService};
pub use error::{QshareError, Result};
pub use server::{Server, ServerHandle};
pub use state::AppState;
pub use stats::{Stats, StatsSnapshot};

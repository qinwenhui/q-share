// Server configuration shared by CLI/GUI/TUI frontends.
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub root: PathBuf,
    pub host: IpAddr,
    pub port: u16,
    pub show_hidden: bool,
    pub cache_ttl_secs: u64,
    pub max_upload: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 8888,
            show_hidden: false,
            cache_ttl_secs: 5,
            max_upload: 0,
        }
    }
}

impl ServerConfig {
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn url(&self) -> String {
        let host = match self.host {
            IpAddr::V4(v4) if v4.is_unspecified() => "127.0.0.1".to_string(),
            IpAddr::V6(v6) if v6.is_unspecified() => "[::1]".to_string(),
            _ => self.host.to_string(),
        };
        format!("http://{}:{}", host, self.port)
    }
}

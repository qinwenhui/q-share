//! mDNS responder wrapper around [`mdns_sd`]. Drop the returned
//! [`MdnsService`] to send a goodbye packet and shut down the daemon thread.

use std::net::IpAddr;

use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::error::{QshareError, Result};

/// Owns the daemon thread + registered service. Drop → unregister + shutdown.
pub struct MdnsService {
    #[allow(dead_code)]
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsService {
    /// Register `_qshare._tcp.local.` on the given port. `root_name` is the
    /// human-readable label included in TXT (e.g. "Documents").
    pub fn register(port: u16, root_name: &str) -> Result<Self> {
        let daemon =
            ServiceDaemon::new().map_err(|e| QshareError::Internal(format!("mdns daemon: {e}")))?;

        let host = hostname();
        let instance = format!("qshare-{host}");

        // Best IP — the LAN IP we already discovered, falling back to all-zero.
        let ip: IpAddr = local_ip().unwrap_or(IpAddr::from([0, 0, 0, 0]));

        let service_info = ServiceInfo::new(
            "_qshare._tcp.local.",
            &instance,
            &format!("{host}.local."),
            ip,
            port,
            &[("v", "1"), ("path", root_name)][..],
        )
        .map_err(|e| QshareError::Internal(format!("mdns service: {e}")))?;

        let fullname = service_info.get_fullname().to_string();
        daemon
            .register(service_info)
            .map_err(|e| QshareError::Internal(format!("mdns register: {e}")))?;

        tracing::info!(name = %fullname, port, "mDNS: advertising as {}", fullname);

        Ok(Self { daemon, fullname })
    }

    pub fn fullname(&self) -> &str {
        &self.fullname
    }
}

impl Drop for MdnsService {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

fn hostname() -> String {
    let raw = std::env::var("HOSTNAME")
        .ok()
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "qshare".to_string());
    // Sanitize: keep only alnum + dash, trim trailing `.local.`
    let trimmed = raw.trim_end_matches(".local.").trim_end_matches(".local");
    let cleaned: String = trimmed
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "qshare".into()
    } else {
        cleaned
    }
}

fn local_ip() -> Option<IpAddr> {
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip())
}

/// Short mDNS label for a shared path — its last component.
pub fn root_label(root: &std::path::Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "share".into())
}

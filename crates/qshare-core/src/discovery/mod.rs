//! mDNS / DNS-SD service registration for LAN discovery.
//!
//! Registers `_qshare._tcp.local.` so other devices on the LAN can discover
//! the running server by name (e.g. `qshare-myhost.local.:8888`).
//!
//! TXT records:
//!   - `path` — root directory name (so peers can show "Documents" instead of
//!     just "q-share on myhost").
//!   - `v` — protocol version, currently `1`.
//!
//! Browsing is intentionally not included in this MVP — clients just need to
//! reach the URL printed by the CLI/GUI/TUI.

pub mod mdns;

pub use mdns::{root_label, MdnsService};

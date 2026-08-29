//! Embeds the built SolidJS SPA (`web/dist/`) into the binary so a single
//! executable can serve the frontend without an external file server.

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
#[include = "*"]
#[exclude = "*.map"]
pub struct Assets;

impl Assets {
    /// Look up an embedded asset by URL-style path (e.g. `"index.html"`).
    /// Returns cheap-to-clone `Bytes` (Arc-backed), suitable for axum bodies.
    pub fn read(path: &str) -> Option<bytes::Bytes> {
        let normalized = path.trim_start_matches('/');
        if normalized.is_empty() {
            return Self::read("index.html");
        }
        <Self as RustEmbed>::get(normalized).map(|f| bytes::Bytes::from(f.data.into_owned()))
    }

    /// Whether an asset exists.
    pub fn exists(path: &str) -> bool {
        Self::read(path).is_some()
    }
}

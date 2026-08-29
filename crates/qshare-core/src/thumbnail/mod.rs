pub mod cache;
pub mod image_thumb;

use std::path::Path;

pub use cache::ThumbnailCache;

/// Output of [`generate_thumbnail`]: either JPEG bytes ready to send, or an
/// explanation of why no thumbnail was produced.
pub enum ThumbResult {
    Jpeg(Vec<u8>),
    Unsupported(String),
}

const DEFAULT_WIDTH: u32 = 320;

/// Generate a JPEG thumbnail for an image file. Returns `Unsupported` if
/// the file is not an image or the decoder fails.
pub fn generate_thumbnail(path: &Path, max_w: u32) -> Result<ThumbResult, String> {
    let img = match image::open(path) {
        Ok(img) => img,
        Err(e) => return Err(format!("decode: {e}")),
    };
    let w = if max_w == 0 { DEFAULT_WIDTH } else { max_w };
    let thumb = if img.width() > w {
        img.resize(
            w,
            img.width().max(1) * img.height() / img.width(),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };

    let mut out = Vec::with_capacity(8 * 1024);
    let mut cursor = std::io::Cursor::new(&mut out);
    thumb
        .to_rgb8()
        .write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| format!("encode: {e}"))?;
    Ok(ThumbResult::Jpeg(out))
}

pub fn cache_key(abs_path: &Path, modified: u64, max_w: u32) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(abs_path.to_string_lossy().as_bytes());
    h.update(b"|");
    h.update(modified.to_le_bytes());
    h.update(b"|");
    h.update(max_w.to_le_bytes());
    let digest = h.finalize();
    // hex of first 16 bytes = 32 chars, plenty for cache key
    let mut out = String::with_capacity(32);
    for b in &digest[..16] {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

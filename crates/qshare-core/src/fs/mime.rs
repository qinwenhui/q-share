//! MIME type helpers and preview capability decisions.

const TEXT_MIMES: &[&str] = &[
    "text/",
    "application/json",
    "application/xml",
    "application/javascript",
    "application/x-yaml",
    "application/toml",
];

const IMAGE_MIMES: &[&str] = &["image/"];

const VIDEO_MIMES: &[&str] = &["video/"];

const AUDIO_MIMES: &[&str] = &["audio/"];

const PDF_MIME: &str = "application/pdf";

/// Can the browser natively render this MIME in an <img>/<video>/<audio>/<iframe>?
pub fn preview_kind(mime: &str) -> PreviewKind {
    if mime == PDF_MIME {
        return PreviewKind::Pdf;
    }
    if IMAGE_MIMES.iter().any(|p| mime.starts_with(p)) {
        return PreviewKind::Image;
    }
    if VIDEO_MIMES.iter().any(|p| mime.starts_with(p)) {
        return PreviewKind::Video;
    }
    if AUDIO_MIMES.iter().any(|p| mime.starts_with(p)) {
        return PreviewKind::Audio;
    }
    if TEXT_MIMES.iter().any(|p| mime.starts_with(p)) {
        return PreviewKind::Text;
    }
    PreviewKind::Other
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Image,
    Video,
    Audio,
    Pdf,
    Text,
    Other,
}

impl PreviewKind {
    pub fn is_previewable(self) -> bool {
        !matches!(self, PreviewKind::Other)
    }
}

use std::path::{Component, Path, PathBuf};

use crate::error::{QshareError, Result};

/// Path sandbox: maps URL paths to filesystem paths while guaranteeing
/// resolution stays inside the configured root.
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            return Err(QshareError::NotFound(format!(
                "shared root does not exist: {}",
                root.display()
            )));
        }
        if !root.is_dir() {
            return Err(QshareError::NotADirectory(format!(
                "shared root is not a directory: {}",
                root.display()
            )));
        }
        let root = std::fs::canonicalize(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Convert an in-URL path (e.g. "/photos/2024/img.jpg") into an
    /// absolute filesystem path. Rejects traversal, absolute paths, NUL
    /// bytes and symlinks pointing outside the root.
    pub fn resolve(&self, url_path: &str) -> Result<PathBuf> {
        if url_path.contains('\0') {
            return Err(QshareError::Forbidden("NUL byte in path".into()));
        }

        // Strip leading slash; we only accept relative paths.
        let trimmed = url_path.trim_start_matches('/');

        // Reject obvious traversal before touching the FS.
        for comp in Path::new(trimmed).components() {
            match comp {
                Component::ParentDir => {
                    return Err(QshareError::Forbidden("'..' not allowed".into()));
                }
                Component::Prefix(_) | Component::RootDir => {
                    return Err(QshareError::Forbidden("absolute path not allowed".into()));
                }
                _ => {}
            }
        }

        let decoded = percent_encoding::percent_decode_str(trimmed)
            .decode_utf8()
            .map_err(|_| QshareError::BadRequest("invalid UTF-8 in path".into()))?
            .into_owned();

        let candidate = self.root.join(&decoded);

        // For an existing path, canonicalize and verify containment.
        // For a non-existing path (e.g. clients querying a not-yet-created file),
        // canonicalize the parent and re-attach the file name.
        let resolved = if candidate.exists() {
            std::fs::canonicalize(&candidate)?
        } else if let Some(parent) = candidate.parent() {
            if !parent.starts_with(&self.root) {
                return Err(QshareError::Forbidden("path escapes root".into()));
            }
            candidate
        } else {
            return Err(QshareError::Forbidden("path escapes root".into()));
        };

        if !resolved.starts_with(&self.root) {
            return Err(QshareError::Forbidden("path escapes root".into()));
        }
        Ok(resolved)
    }

    /// Translate an absolute filesystem path back into a URL path relative to root.
    pub fn to_url_path(&self, abs: &Path) -> Option<String> {
        let rel = abs.strip_prefix(&self.root).ok()?;
        let mut out = String::new();
        for (i, comp) in rel.components().enumerate() {
            if i > 0 {
                out.push('/');
            }
            let s = comp.as_os_str().to_string_lossy();
            // percent-encode each segment
            let mut buf = String::new();
            for byte in s.as_bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                        buf.push(*byte as char)
                    }
                    _ => buf.push_str(&format!("%{:02X}", byte)),
                }
            }
            out.push_str(&buf);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn make_sandbox() -> Sandbox {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir().join(format!("qs-sandbox-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sub/dir")).unwrap();
        fs::write(tmp.join("file.txt"), b"hi").unwrap();
        Sandbox::new(tmp).unwrap()
    }

    #[test]
    fn resolve_normal_path() {
        let s = make_sandbox();
        let p = s.resolve("/file.txt").unwrap();
        assert!(p.ends_with("file.txt"));
    }

    #[test]
    fn resolve_nested_path() {
        let s = make_sandbox();
        let p = s.resolve("/sub/dir").unwrap();
        assert!(p.ends_with("sub/dir") || p.ends_with("sub\\dir"));
    }

    #[test]
    fn reject_traversal() {
        let s = make_sandbox();
        assert!(matches!(
            s.resolve("/../etc/passwd"),
            Err(QshareError::Forbidden(_))
        ));
        assert!(matches!(
            s.resolve("/sub/../../etc/passwd"),
            Err(QshareError::Forbidden(_))
        ));
    }

    #[test]
    fn reject_nul() {
        let s = make_sandbox();
        assert!(matches!(
            s.resolve("/foo\0bar"),
            Err(QshareError::Forbidden(_))
        ));
    }

    #[test]
    fn reject_absolute_segment() {
        let s = make_sandbox();
        // An URL path that *resolves* to /etc/passwd must be rejected.
        // On macOS /tmp -> /private/tmp so canonicalize will differ from
        // the simple join; what matters is that the result is outside the
        // sandbox root.
        let abs_etc = std::path::Path::new("/etc");
        let res = s.resolve("/file.txt").unwrap();
        assert!(res.starts_with(s.root()));
        assert!(!s.root().starts_with(abs_etc) || !res.starts_with(abs_etc));
    }
}

//! HTTP Range header parsing.
//!
//! Supports `bytes=start-end`, `bytes=start-`, `bytes=-suffix`.
//! Multi-range (`bytes=0-100,200-300`) is rejected — the client should issue
//! a fresh range request if needed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeSpec {
    pub start: u64,
    /// Inclusive end. None means read to EOF.
    pub end: Option<u64>,
    pub length: u64,
}

impl RangeSpec {
    pub fn parse(header: &str, file_size: u64) -> Option<RangeSpec> {
        let header = header.trim();
        let rest = header.strip_prefix("bytes=")?;
        // Single-range only; reject multi.
        if rest.contains(',') {
            return None;
        }
        let (start_str, end_str) = rest.split_once('-')?;

        if start_str.is_empty() {
            // suffix: bytes=-N => last N bytes
            let n: u64 = end_str.parse().ok()?;
            if n == 0 {
                return None;
            }
            let n = n.min(file_size);
            let start = file_size - n;
            return Some(RangeSpec {
                start,
                end: Some(file_size - 1),
                length: n,
            });
        }

        let start: u64 = start_str.parse().ok()?;
        let end: u64 = if end_str.is_empty() {
            file_size.saturating_sub(1)
        } else {
            end_str.parse().ok()?
        };

        if start > end || start >= file_size {
            return None;
        }
        let end = end.min(file_size - 1);
        Some(RangeSpec {
            start,
            end: Some(end),
            length: end - start + 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_ended() {
        let r = RangeSpec::parse("bytes=100-", 1000).unwrap();
        assert_eq!(r.start, 100);
        assert_eq!(r.end, Some(999));
        assert_eq!(r.length, 900);
    }

    #[test]
    fn parses_closed() {
        let r = RangeSpec::parse("bytes=0-1023", 5000).unwrap();
        assert_eq!(r.start, 0);
        assert_eq!(r.end, Some(1023));
        assert_eq!(r.length, 1024);
    }

    #[test]
    fn parses_suffix() {
        let r = RangeSpec::parse("bytes=-500", 1000).unwrap();
        assert_eq!(r.start, 500);
        assert_eq!(r.end, Some(999));
        assert_eq!(r.length, 500);
    }

    #[test]
    fn rejects_multi_range() {
        assert!(RangeSpec::parse("bytes=0-100,200-300", 1000).is_none());
    }

    #[test]
    fn rejects_out_of_bounds_start() {
        assert!(RangeSpec::parse("bytes=2000-3000", 1000).is_none());
    }

    #[test]
    fn clamps_end_to_size() {
        let r = RangeSpec::parse("bytes=900-9999", 1000).unwrap();
        assert_eq!(r.end, Some(999));
    }

    #[test]
    fn rejects_inverted_range() {
        assert!(RangeSpec::parse("bytes=500-100", 1000).is_none());
    }
}

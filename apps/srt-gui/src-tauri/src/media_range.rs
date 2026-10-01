/// Parse one HTTP byte range, rejecting empty files and unsatisfiable ranges.
pub fn parse_byte_range(header: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let range = header.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        return (suffix > 0).then(|| (size.saturating_sub(suffix), size - 1));
    }
    let start: u64 = start.parse().ok()?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().ok()?.min(size - 1)
    };
    (start < size && start <= end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valid_ranges() {
        assert_eq!(parse_byte_range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(parse_byte_range("bytes=2-", 10), Some((2, 9)));
        assert_eq!(parse_byte_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_byte_range("bytes=-20", 10), Some((0, 9)));
        assert_eq!(parse_byte_range("bytes=2-99", 10), Some((2, 9)));
    }
    #[test]
    fn invalid_ranges() {
        for header in [
            "bytes=10-",
            "bytes=5-2",
            "bytes=-0",
            "bytes=x-y",
            "bytes=0-1,3-4",
            "items=0-1",
        ] {
            assert_eq!(parse_byte_range(header, 10), None, "{header}");
        }
        assert_eq!(parse_byte_range("bytes=0-", 0), None);
    }
}

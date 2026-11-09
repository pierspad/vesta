//! Encoding-tolerant reader for subtitle files.
//!
//! Subtitle files in the wild use a wide variety of character encodings:
//! UTF-8 with or without BOM, UTF-16 (LE/BE, with or without BOM), and legacy
//! single-byte or multi-byte code pages (Windows-1252, GBK, Shift_JIS, etc.).
//!
//! Decoding follows four ordered stages:
//!
//! 1. **BOM sniffing** — UTF-8, UTF-16LE, or UTF-16BE declared by a byte-order mark;
//! 2. **BOM-less UTF-16** — detected from the even/odd NUL byte distribution;
//! 3. **Strict UTF-8** — zero-copy validation when the byte stream is valid UTF-8;
//! 4. **Statistical detection** — [`chardetng`] selects the legacy code page and
//!    [`encoding_rs`] decodes it into UTF-8.
//!
//! The output is always a clean UTF-8 `String` without a leading BOM.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};

/// Reads a text file (typically a subtitle file) with automatic encoding detection.
/// Unmappable bytes in legacy encodings are replaced with `U+FFFD`.
pub fn read_text_auto<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let bytes = fs::read(path)
        .with_context(|| format!("Failed to read file {}", path.display()))?;
    Ok(decode_auto(&bytes))
}

/// Decodes a byte buffer into a UTF-8 `String` using automatic encoding detection.
pub fn decode_auto(bytes: &[u8]) -> String {
    // 1. Explicit BOM: decode() strips the BOM automatically.
    if Encoding::for_bom(bytes).is_some() {
        let (text, _, _) = encoding_rs::UTF_8.decode(bytes);
        return text.into_owned();
    }

    // 2. Check UTF-16 before UTF-8: ASCII UTF-16 bytes (including NUL)
    // are also valid UTF-8, but must still be decoded as UTF-16.
    if let Some(enc) = sniff_bomless_utf16(bytes) {
        let (text, _) = enc.decode_without_bom_handling(bytes);
        return text.into_owned();
    }

    // 3. Valid UTF-8 -> no conversion required.
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }

    // 4. Legacy code page: statistical detection. ISO-2022-JP is allowed for local
    // files, and UTF-8 detection is disabled because step 3 already ruled it out.
    let mut detector = EncodingDetector::new(Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, Utf8Detection::Deny);
    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

/// Heuristic for BOM-less UTF-16: in predominantly Latin text, the high bytes of
/// 16-bit code units are mostly NUL (`>= 30%` on one parity and `<= 5%` on the other).
fn sniff_bomless_utf16(bytes: &[u8]) -> Option<&'static Encoding> {
    if bytes.len() < 4 {
        return None;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    let (mut nul_even, mut nul_odd) = (0usize, 0usize);
    let (pairs, _) = sample.as_chunks::<2>();
    for pair in pairs {
        nul_even += usize::from(pair[0] == 0);
        nul_odd += usize::from(pair[1] == 0);
    }
    let pair_count = pairs.len();
    match () {
        _ if nul_odd * 10 >= pair_count * 3 && nul_even * 20 <= pair_count => Some(UTF_16LE),
        _ if nul_even * 10 >= pair_count * 3 && nul_odd * 20 <= pair_count => Some(UTF_16BE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "1\n00:00:01,000 --> 00:00:02,000\nCiao, però…\n";

    #[test]
    fn utf8_plain_roundtrips() {
        assert_eq!(decode_auto(SAMPLE.as_bytes()), SAMPLE);
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(SAMPLE.as_bytes());
        assert_eq!(decode_auto(&bytes), SAMPLE);
    }

    #[test]
    fn utf16le_with_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(SAMPLE.encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(decode_auto(&bytes), SAMPLE);
    }

    #[test]
    fn utf16le_without_bom() {
        let bytes: Vec<u8> = SAMPLE.encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_auto(&bytes), SAMPLE);
    }

    #[test]
    fn utf16be_without_bom() {
        let bytes: Vec<u8> = SAMPLE.encode_utf16().flat_map(u16::to_be_bytes).collect();
        assert_eq!(decode_auto(&bytes), SAMPLE);
    }

    #[test]
    fn ascii_only_bomless_utf16_is_not_mistaken_for_utf8() {
        let text = "1\n00:00:01,000 --> 00:00:02,000\nHello!\n";
        for bytes in [
            text.encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
            text.encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>(),
        ] {
            assert!(std::str::from_utf8(&bytes).is_ok());
            assert_eq!(decode_auto(&bytes), text);
        }
    }

    #[test]
    fn utf16_bom_controls_endianness_and_replaces_malformed_units() {
        for (bom, little_endian) in [([0xff, 0xfe], true), ([0xfe, 0xff], false)] {
            let mut bytes = bom.to_vec();
            for unit in [0x0041u16, 0xd83d, 0xde00, 0xd800, 0x0042] {
                bytes.extend(if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            bytes.push(0x41); // Incomplete final code unit.
            assert_eq!(decode_auto(&bytes), "A😀�B�");
        }
    }

    #[test]
    fn empty_short_and_utf8_with_isolated_nul_remain_unchanged() {
        for text in ["", "a", "é", "Hello\0world", "日本語 😀"] {
            assert_eq!(decode_auto(text.as_bytes()), text);
        }
    }

    #[test]
    fn windows_1252_accents_survive() {
        // "però" in Windows-1252: 'ò' = 0xF2 (UTF-8 invalido → path chardetng).
        let text = "1\n00:00:01,000 --> 00:00:02,000\nPer\u{f2} s\u{ec}, andr\u{e0} l\u{e0}. \
                    Citt\u{e0} unit\u{e0} variet\u{e0} met\u{e0} qualit\u{e0} verit\u{e0}.\n";
        let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(text);
        assert_eq!(decode_auto(&bytes), text);
    }

    #[test]
    fn gbk_chinese_survives() {
        // Il caso segnalato dall'utente: sottotitoli cinesi in GBK.
        let text = "1\n00:00:01,000 --> 00:00:02,000\n\u{4f60}\u{597d}\u{ff0c}\u{4e16}\u{754c}\
                    \u{3002}\u{8fd9}\u{662f}\u{4e00}\u{4e2a}\u{5b57}\u{5e55}\u{6587}\u{4ef6}\u{3002}\n";
        let (bytes, _, _) = encoding_rs::GBK.encode(text);
        assert_eq!(decode_auto(&bytes), text);
    }
}

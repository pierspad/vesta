//! Lettura *encoding-tolerant* dei file di sottotitoli.
//!
//! I sottotitoli "in the wild" arrivano in ogni encoding immaginabile:
//! UTF-8 con/senza BOM, UTF-16 (con/senza BOM), code page legacy
//! (Windows-1252, GBK, Shift_JIS, …). Leggerli come UTF-8 strict o
//! fallisce o produce mojibake (il classico file che "contiene cinese").
//!
//! Strategia di decodifica, dalla più certa alla più euristica:
//!
//! 1. **BOM sniffing** — UTF-8 / UTF-16LE / UTF-16BE dichiarati dal BOM;
//! 2. **UTF-8 strict** — se i byte sono UTF-8 valido, è UTF-8 (falsi
//!    positivi statisticamente trascurabili);
//! 3. **UTF-16 senza BOM** — riconosciuto dalla distribuzione dei byte
//!    NUL su posizioni pari/dispari;
//! 4. **rilevamento statistico** — [`chardetng`] sceglie la code page
//!    (GBK, Windows-1252, …) e [`encoding_rs`] decodifica.
//!
//! L'output è sempre una `String` UTF-8 pulita, senza BOM.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};

/// Legge un file di testo (tipicamente un sottotitolo) rilevandone
/// automaticamente l'encoding. Non fallisce mai per problemi di encoding:
/// nel caso peggiore i byte non mappabili diventano U+FFFD.
pub fn read_text_auto<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let bytes = fs::read(path)
        .with_context(|| format!("Impossibile leggere il file {}", path.display()))?;
    Ok(decode_auto(&bytes))
}

/// Decodifica un buffer di byte in `String` UTF-8, rilevando l'encoding.
pub fn decode_auto(bytes: &[u8]) -> String {
    // 1. BOM esplicito: fidati di quello (decode() lo rimuove).
    if Encoding::for_bom(bytes).is_some() {
        let (text, _, _) = encoding_rs::UTF_8.decode(bytes); // BOM-sniffing interno
        return text.into_owned();
    }

    // 2. UTF-8 valido → nessuna conversione necessaria.
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }

    // 3. UTF-16 senza BOM (Notepad/strumenti Windows d'epoca).
    if let Some(enc) = sniff_bomless_utf16(bytes) {
        let (text, _) = enc.decode_without_bom_handling(bytes);
        return text.into_owned();
    }

    // 4. Code page legacy: rilevamento statistico. ISO-2022-JP è permesso
    // (siamo file locali, non contenuto web scriptabile); UTF-8 è già stato
    // escluso dal fast path, quindi Deny evita falsi positivi ridondanti.
    let mut detector = EncodingDetector::new(Iso2022JpDetection::Allow);
    detector.feed(bytes, true);
    let encoding = detector.guess(None, Utf8Detection::Deny);
    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

/// Euristica per UTF-16 privo di BOM: nel testo prevalentemente latino i
/// byte "alti" delle coppie sono quasi tutti NUL. Richiede ≥30% di NUL su
/// una metà delle posizioni e ~nessuno sull'altra.
fn sniff_bomless_utf16(bytes: &[u8]) -> Option<&'static Encoding> {
    if bytes.len() < 4 {
        return None;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    let (mut nul_even, mut nul_odd) = (0usize, 0usize);
    for pair in sample.chunks_exact(2) {
        nul_even += usize::from(pair[0] == 0);
        nul_odd += usize::from(pair[1] == 0);
    }
    let pairs = sample.len() / 2;
    match () {
        _ if nul_odd * 10 >= pairs * 3 && nul_even * 20 <= pairs => Some(UTF_16LE),
        _ if nul_even * 10 >= pairs * 3 && nul_odd * 20 <= pairs => Some(UTF_16BE),
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

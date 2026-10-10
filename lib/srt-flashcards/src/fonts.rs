use crate::types::FlashcardConfig;
use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};
use srt_download::download_to;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

pub const DEFAULT_FONT_STACK: &str =
    r#""Noto Sans", -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif"#;

#[derive(Debug, Clone, Copy)]
pub struct FontCatalogEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub language_name: &'static str,
    pub target_languages: &'static [&'static str],
    pub filename: &'static str,
    pub url: &'static str,
    pub approx_size: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontStatusInfo {
    pub id: String,
    pub name: String,
    pub language_name: String,
    pub target_languages: Vec<String>,
    pub filename: String,
    pub approx_size: String,
    pub downloaded: bool,
}

pub const FONT_CATALOG: &[FontCatalogEntry] = &[
    FontCatalogEntry {
        id: "noto-sans-jp",
        name: "Noto Sans JP",
        language_name: "Japanese",
        target_languages: &["ja", "jpn"],
        filename: "_NotoSansJP.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf",
        approx_size: "~11 MB",
    },
    FontCatalogEntry {
        id: "noto-sans-sc",
        name: "Noto Sans SC",
        language_name: "Chinese Simplified",
        target_languages: &["zh", "zh-cn", "chs"],
        filename: "_NotoSansSC.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf",
        approx_size: "~12 MB",
    },
    FontCatalogEntry {
        id: "noto-sans-tc",
        name: "Noto Sans TC",
        language_name: "Chinese Traditional",
        target_languages: &["zh-tw", "zh-hk", "zh-hant", "cht"],
        filename: "_NotoSansTC.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosanstc/NotoSansTC%5Bwght%5D.ttf",
        approx_size: "~13 MB",
    },
    FontCatalogEntry {
        id: "noto-sans-kr",
        name: "Noto Sans KR",
        language_name: "Korean",
        target_languages: &["ko", "kor"],
        filename: "_NotoSansKR.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosanskr/NotoSansKR%5Bwght%5D.ttf",
        approx_size: "~13 MB",
    },
    FontCatalogEntry {
        id: "noto-sans-arabic",
        name: "Noto Sans Arabic",
        language_name: "Arabic",
        target_languages: &["ar", "ara", "arb"],
        filename: "_NotoSansArabic.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansarabic/NotoSansArabic%5Bwdth%2Cwght%5D.ttf",
        approx_size: "~850 KB",
    },
    FontCatalogEntry {
        id: "noto-sans-hebrew",
        name: "Noto Sans Hebrew",
        language_name: "Hebrew",
        target_languages: &["he", "heb"],
        filename: "_NotoSansHebrew.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosanshebrew/NotoSansHebrew%5Bwdth%2Cwght%5D.ttf",
        approx_size: "~110 KB",
    },
    FontCatalogEntry {
        id: "noto-sans-thai",
        name: "Noto Sans Thai",
        language_name: "Thai",
        target_languages: &["th", "tha"],
        filename: "_NotoSansThai.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansthai/NotoSansThai%5Bwdth%2Cwght%5D.ttf",
        approx_size: "~220 KB",
    },
    FontCatalogEntry {
        id: "noto-sans-devanagari",
        name: "Noto Sans Devanagari",
        language_name: "Hindi / Devanagari",
        target_languages: &["hi", "hin"],
        filename: "_NotoSansDevanagari.ttf",
        url: "https://raw.githubusercontent.com/google/fonts/main/ofl/notosansdevanagari/NotoSansDevanagari%5Bwdth%2Cwght%5D.ttf",
        approx_size: "~650 KB",
    },
];

pub fn get_fonts_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".cache")
    });
    Ok(cache_dir.join("vesta").join("fonts"))
}

pub fn font_file_path_for_entry(entry: &FontCatalogEntry) -> Result<PathBuf> {
    let dir = get_fonts_cache_dir()?;
    Ok(dir.join(entry.filename))
}

pub fn font_file_path(font_id: &str) -> Result<PathBuf> {
    let entry = FONT_CATALOG
        .iter()
        .find(|e| e.id == font_id)
        .ok_or_else(|| anyhow!("Unknown font id: {font_id}"))?;
    font_file_path_for_entry(entry)
}

/// Normalize script variants and ISO-639 aliases identically for CSS and embedding.
fn normalized_font_language(lang: &str) -> String {
    let lower = lang.trim().to_lowercase().replace('_', "-");
    if lower == "cht"
        || ["zh-hant", "zh-tw", "zh-hk"]
            .iter()
            .any(|prefix| lower == *prefix || lower.starts_with(&format!("{prefix}-")))
    {
        return "zh-tw".into();
    }
    let primary = lower.split('-').next().unwrap_or("");
    match primary {
        "jpn" => "ja",
        "kor" => "ko",
        "chi" | "zho" | "cmn" | "chs" => "zh",
        "ara" | "arb" => "ar",
        "heb" => "he",
        "tha" => "th",
        "hin" => "hi",
        "gre" | "ell" => "el",
        "rus" => "ru",
        "ukr" => "uk",
        _ => primary,
    }
    .into()
}

pub fn font_entry_for_lang(lang: &str) -> Option<&'static FontCatalogEntry> {
    let lower = normalized_font_language(lang);
    // Sub-tags first
    if lower == "zh-tw" || lower == "zh-hk" || lower == "zh-hant" || lower == "cht" {
        return FONT_CATALOG.iter().find(|e| e.id == "noto-sans-tc");
    }
    let primary = lower.split(&['-', '_'][..]).next().unwrap_or(&lower);
    FONT_CATALOG
        .iter()
        .find(|e| e.target_languages.contains(&primary))
}

pub fn get_downloaded_font_for_lang(lang: &str) -> Option<(&'static FontCatalogEntry, PathBuf)> {
    let entry = font_entry_for_lang(lang)?;
    let path = font_file_path_for_entry(entry).ok()?;
    if path.exists() {
        Some((entry, path))
    } else {
        None
    }
}

pub fn list_fonts() -> Result<Vec<FontStatusInfo>> {
    Ok(FONT_CATALOG.iter().map(font_status).collect())
}

pub fn font_status(entry: &FontCatalogEntry) -> FontStatusInfo {
    let downloaded = font_file_path_for_entry(entry).is_ok_and(|path| path.is_file());
    FontStatusInfo {
        id: entry.id.to_owned(),
        name: entry.name.to_owned(),
        language_name: entry.language_name.to_owned(),
        target_languages: entry
            .target_languages
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        filename: entry.filename.to_owned(),
        approx_size: entry.approx_size.to_owned(),
        downloaded,
    }
}

pub fn delete_font(font_id: &str) -> Result<bool> {
    let path = font_file_path(font_id)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove font file: {}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn download_font<F>(
    font_id: &str,
    progress_callback: F,
    cancel_token: Option<&CancellationToken>,
) -> Result<PathBuf>
where
    F: Fn(u32) + Send + 'static,
{
    let entry = FONT_CATALOG
        .iter()
        .find(|e| e.id == font_id)
        .ok_or_else(|| anyhow!("Unknown font id: {font_id}"))?;
    let path = font_file_path_for_entry(entry)?;

    download_to(entry.url, &path, progress_callback, cancel_token).await
}

/// Return CSS `font-family` stack for given target language code.
/// Matches case-insensitively and handles sub-tags (e.g. `zh-tw` before `zh`).
pub fn font_stack_for(lang: &str) -> &'static str {
    let lower = normalized_font_language(lang);

    // Check specific sub-tags first
    if lower == "zh-tw" || lower == "zh-hk" || lower == "zh-hant" || lower == "cht" {
        return r#""Noto Sans CJK TC", "Noto Sans TC", "PingFang TC", "Microsoft JhengHei", sans-serif"#;
    }

    let primary = lower.split(&['-', '_'][..]).next().unwrap_or(&lower);

    match primary {
        "ja" => {
            r#""Noto Sans CJK JP", "Noto Sans JP", "Hiragino Sans", "Yu Gothic", Meiryo, sans-serif"#
        }
        "zh" => {
            r#""Noto Sans CJK SC", "Noto Sans SC", "Source Han Sans SC", "PingFang SC", "Microsoft YaHei", sans-serif"#
        }
        "ko" => {
            r#""Noto Sans CJK KR", "Noto Sans KR", "Apple SD Gothic Neo", "Malgun Gothic", sans-serif"#
        }
        "ar" => r#""Noto Naskh Arabic", "Noto Sans Arabic", "Geeza Pro", "Segoe UI", sans-serif"#,
        "he" => r#""Noto Sans Hebrew", "Arial Hebrew", "Segoe UI", sans-serif"#,
        "th" => r#""Noto Sans Thai", Thonburi, "Leelawadee UI", sans-serif"#,
        "hi" => r#""Noto Sans Devanagari", "Kohinoor Devanagari", "Nirmala UI", sans-serif"#,
        "el" | "ru" | "uk" => r#""Noto Sans", "Segoe UI", "Helvetica Neue", sans-serif"#,
        _ => DEFAULT_FONT_STACK,
    }
}

pub fn maybe_prepend_font_vars(base_css: &str, config: &FlashcardConfig) -> String {
    if !config.auto_card_font {
        return base_css.to_string();
    }

    let target_lang = config.target_language.as_deref().unwrap_or("");
    let stack = if !target_lang.trim().is_empty() {
        font_stack_for(target_lang)
    } else {
        DEFAULT_FONT_STACK
    };

    let downloaded_font = if config.embed_card_font && !target_lang.trim().is_empty() {
        get_downloaded_font_for_lang(target_lang)
    } else {
        None
    };

    let font_block = if let Some((entry, _)) = downloaded_font {
        format!(
            "/* vesta:font-start */\n@font-face {{\n  font-family: \"VestaTargetFont\";\n  src: url(\"{}\") format(\"truetype\");\n}}\n:root {{\n  --vesta-target-font: \"VestaTargetFont\", {};\n}}\n.card {{\n  font-family: var(--vesta-target-font);\n}}\n/* vesta:font-end */",
            entry.filename, stack
        )
    } else {
        format!(
            "/* vesta:font-start */\n:root {{\n  --vesta-target-font: {};\n}}\n.card {{\n  font-family: var(--vesta-target-font);\n}}\n/* vesta:font-end */",
            stack
        )
    };

    if let (Some(start_idx), Some(end_idx)) = (
        base_css.find("/* vesta:font-start */"),
        base_css.find("/* vesta:font-end */"),
    ) {
        let end_block = end_idx + "/* vesta:font-end */".len();
        let mut result = String::with_capacity(base_css.len() + font_block.len());
        result.push_str(&base_css[..start_idx]);
        result.push_str(&font_block);
        result.push_str(&base_css[end_block..]);
        result
    } else {
        format!("{}\n\n{}", font_block, base_css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_script_variants_match_the_same_font_and_css() {
        for lang in ["zh_Hant_TW", "zh-Hant-HK", "zh_HK", "cht"] {
            assert_eq!(font_entry_for_lang(lang).unwrap().id, "noto-sans-tc");
            assert!(font_stack_for(lang).contains("CJK TC"));
        }
        for (lang, id, css) in [
            ("jpn", "noto-sans-jp", "CJK JP"),
            ("kor", "noto-sans-kr", "CJK KR"),
            ("zho", "noto-sans-sc", "CJK SC"),
            ("arb", "noto-sans-arabic", "Arabic"),
        ] {
            assert_eq!(font_entry_for_lang(lang).unwrap().id, id);
            assert!(font_stack_for(lang).contains(css));
        }
        assert!(font_entry_for_lang("en").is_none());
        assert!(font_entry_for_lang("de").is_none());
    }

    #[test]
    fn test_font_stack_matching() {
        assert!(font_stack_for("ja").contains("Noto Sans CJK JP"));
        assert!(font_stack_for("zh-TW").contains("Noto Sans CJK TC"));
        assert!(font_stack_for("ZH-cn").contains("Noto Sans CJK SC"));
        assert!(font_stack_for("ko").contains("Noto Sans CJK KR"));
        assert!(font_stack_for("ar").contains("Noto Naskh Arabic"));
        assert_eq!(font_stack_for("xyz_unknown"), DEFAULT_FONT_STACK);
    }

    #[test]
    fn test_font_catalog_matching() {
        assert_eq!(font_entry_for_lang("ja").unwrap().id, "noto-sans-jp");
        assert_eq!(font_entry_for_lang("zh-TW").unwrap().id, "noto-sans-tc");
        assert_eq!(font_entry_for_lang("zh-CN").unwrap().id, "noto-sans-sc");
        assert_eq!(font_entry_for_lang("ar").unwrap().id, "noto-sans-arabic");
        assert!(font_entry_for_lang("unknown_lang").is_none());
    }

    #[test]
    fn test_css_injection_and_idempotency() {
        let config = FlashcardConfig {
            target_language: Some("ja".to_string()),
            auto_card_font: true,
            embed_card_font: false, // test stack without local file
            ..Default::default()
        };

        let base_css = ".card { color: white; }";
        let injected = maybe_prepend_font_vars(base_css, &config);
        assert!(injected.contains("/* vesta:font-start */"));
        assert!(injected.contains("Noto Sans CJK JP"));
        assert!(injected.contains(".card { color: white; }"));

        // Idempotency check: running it again on injected CSS replaces the block
        let config_zh = FlashcardConfig {
            target_language: Some("zh".to_string()),
            auto_card_font: true,
            embed_card_font: false,
            ..Default::default()
        };
        let reinjected = maybe_prepend_font_vars(&injected, &config_zh);
        assert!(reinjected.contains("Noto Sans CJK SC"));
        assert!(!reinjected.contains("Noto Sans CJK JP"));
        assert_eq!(reinjected.matches("/* vesta:font-start */").count(), 1);

        // Disabled check
        let config_disabled = FlashcardConfig {
            auto_card_font: false,
            ..Default::default()
        };
        assert_eq!(
            maybe_prepend_font_vars(base_css, &config_disabled),
            base_css
        );
    }
}

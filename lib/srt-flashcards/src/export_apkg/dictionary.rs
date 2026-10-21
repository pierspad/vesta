//! Optional lookup adapter; source text stays unchanged in the note database.
pub(super) fn lookup_script(language: Option<&str>) -> String {
    let language = language.unwrap_or("");
    // Only a BCP-47-like tag may enter the script, never arbitrary template HTML.
    let language = if !language.is_empty()
        && language.len() <= 35
        && language
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        language
    } else {
        "und"
    };
    let language_json = serde_json::to_string(language).unwrap();
    format!(
        "<script>\n{}\n</script>",
        include_str!("dictionary.js").replace("__VESTA_LANGUAGE__", &language_json)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_opt_in_and_reaches_both_model_templates() {
        let mut config = crate::types::FlashcardConfig {
            dictionary_lookup: false,
            ..Default::default()
        };
        let (_, models, _, _) = super::super::model::build_collection_metadata(&config, 1, 2, 3);
        assert!(!models.contains("vestaDictionary"));
        config.dictionary_lookup = true;
        config.target_language = Some("ja".into());
        let (_, models, _, _) = super::super::model::build_collection_metadata(&config, 1, 2, 3);
        let models: serde_json::Value = serde_json::from_str(&models).unwrap();
        for side in ["qfmt", "afmt"] {
            let template = models["3"]["tmpls"][0][side].as_str().unwrap();
            assert!(template.contains("var language = \"ja\""));
            assert!(template.contains("vestaDictionary"));
            assert!(!template.contains("__VESTA_LANGUAGE__"));
        }
    }

    #[test]
    fn language_cannot_inject_html_or_script() {
        let invalid = lookup_script(Some("</script><img src=x onerror=alert(1)>"));
        assert!(!invalid.contains("<img"));
        assert!(invalid.contains("var language = \"und\""));
        assert!(lookup_script(Some("zh-tw")).contains("var language = \"zh-tw\""));
    }
}

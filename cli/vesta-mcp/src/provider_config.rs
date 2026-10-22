//! Server-side provider profiles for MCP translation and transcription tools.
//!
//! Config files contain environment variable *names*, never provider secrets.
//! Resolved profiles intentionally do not implement `Debug` or `Serialize`.

use serde::Deserialize;
use srt_transcribe::pipeline::TranscriptionConfig;
use srt_translate::TierEntry;
use std::{collections::BTreeMap, path::Path};

#[derive(Clone)]
pub struct TranslationProfile {
    pub tiers: Vec<Vec<TierEntry>>,
    pub batch_size: usize,
    pub resume_overlap: usize,
    pub title_context: Option<String>,
}

#[derive(Clone)]
pub struct TranscriptionProfile {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub translate_to_english: bool,
    pub word_timestamps: bool,
    pub quality: bool,
    pub vad: bool,
    pub use_gpu: bool,
    pub max_segment_length: u32,
    pub allow_model_download: bool,
}

impl TranscriptionProfile {
    pub fn config(
        &self,
        input_path: String,
        output_path: String,
        language: Option<String>,
    ) -> TranscriptionConfig {
        TranscriptionConfig {
            input_path,
            output_path,
            model: self.model.clone(),
            language: language.unwrap_or_else(|| "auto".to_owned()),
            translate_to_english: self.translate_to_english,
            word_timestamps: self.word_timestamps,
            max_segment_length: self.max_segment_length,
            provider: Some(self.provider.clone()),
            api_key: self.api_key.clone(),
            api_url: self.api_url.clone(),
            quality: self.quality,
            vad: self.vad,
            vad_model_id: None,
            vad_custom_path: None,
            use_gpu: self.use_gpu,
        }
    }
}

#[derive(Clone, Default)]
pub struct ProviderConfig {
    translation: BTreeMap<String, TranslationProfile>,
    transcription: BTreeMap<String, TranscriptionProfile>,
}

impl ProviderConfig {
    /// Load JSON and resolve environment variable references using the process
    /// environment. Errors identify fields or variable names, never values.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        Self::load_with(path, |name| std::env::var(name).ok())
    }

    /// Injectable resolver keeps configuration tests independent of process
    /// environment mutation and makes secret handling straightforward to test.
    pub fn load_with(
        path: impl AsRef<Path>,
        resolve_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|_| "Could not read provider configuration file".to_owned())?;
        Self::parse_with(&text, resolve_env)
    }

    pub fn parse_with(
        text: &str,
        resolve_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let raw: RawProviderConfig = serde_json::from_str(text)
            .map_err(|_| "Invalid provider configuration JSON".to_owned())?;
        let mut resolved = Self::default();
        for (id, profile) in raw.translation.profiles {
            validate_id(&id)?;
            resolved
                .translation
                .insert(id, resolve_translation(profile, &resolve_env)?);
        }
        for (id, profile) in raw.transcription.profiles {
            validate_id(&id)?;
            resolved
                .transcription
                .insert(id, resolve_transcription(profile, &resolve_env)?);
        }
        Ok(resolved)
    }

    pub fn translation_profile(&self, id: &str) -> Result<&TranslationProfile, String> {
        self.translation
            .get(id)
            .ok_or_else(|| "Unknown translation profile".to_owned())
    }

    pub fn transcription_profile(&self, id: &str) -> Result<&TranscriptionProfile, String> {
        self.transcription
            .get(id)
            .ok_or_else(|| "Unknown transcription profile".to_owned())
    }

    pub fn translation_profile_ids(&self) -> Vec<&str> {
        self.translation.keys().map(String::as_str).collect()
    }

    pub fn transcription_profile_ids(&self) -> Vec<&str> {
        self.transcription.keys().map(String::as_str).collect()
    }

    pub fn translation_available(&self) -> bool {
        !self.translation.is_empty()
    }

    pub fn transcription_available(&self) -> bool {
        !self.transcription.is_empty()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProviderConfig {
    #[serde(default)]
    translation: RawProfiles<RawTranslationProfile>,
    #[serde(default)]
    transcription: RawProfiles<RawTranscriptionProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct RawProfiles<T> {
    #[serde(default)]
    profiles: BTreeMap<String, T>,
}

impl<T> Default for RawProfiles<T> {
    fn default() -> Self {
        Self {
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTranslationProfile {
    tiers: Vec<Vec<RawTranslationTier>>,
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    #[serde(default = "default_resume_overlap")]
    resume_overlap: usize,
    #[serde(default)]
    title_context: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTranslationTier {
    provider: String,
    model: String,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    rpm: Option<u32>,
    #[serde(default)]
    max_requests: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTranscriptionProfile {
    provider: String,
    model: String,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    translate_to_english: bool,
    #[serde(default)]
    word_timestamps: bool,
    #[serde(default)]
    quality: bool,
    #[serde(default)]
    vad: bool,
    #[serde(default = "default_true")]
    use_gpu: bool,
    #[serde(default)]
    max_segment_length: u32,
    #[serde(default)]
    allow_model_download: bool,
}

fn default_batch_size() -> usize {
    25
}
fn default_resume_overlap() -> usize {
    2
}
fn default_true() -> bool {
    true
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        return Err("Profile IDs must be 1-64 ASCII letters, digits, '.', '_' or '-'".into());
    }
    Ok(())
}

fn api_key(
    name: Option<String>,
    required: bool,
    resolve_env: &impl Fn(&str) -> Option<String>,
) -> Result<Option<String>, String> {
    match name {
        Some(name) => {
            if name.is_empty()
                || !name
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err("Credential references must be environment variable names".into());
            }
            let value = resolve_env(&name)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "Credential environment variable is unset or empty".to_owned())?;
            Ok(Some(value))
        }
        None if required => Err("This provider requires an api_key_env reference".into()),
        None => Ok(None),
    }
}

fn validate_api_url(value: &str, label: &str) -> Result<(), String> {
    let value = value.trim();
    let url = reqwest::Url::parse(value)
        .map_err(|_| format!("{label} must be a valid HTTP or HTTPS URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || value.chars().any(char::is_whitespace)
        || value.chars().count() > 2048
    {
        return Err(format!(
            "{label} must be an HTTP or HTTPS URL without embedded credentials, query or fragment"
        ));
    }
    Ok(())
}

fn resolve_translation(
    raw: RawTranslationProfile,
    resolve_env: &impl Fn(&str) -> Option<String>,
) -> Result<TranslationProfile, String> {
    if raw.tiers.is_empty() || raw.tiers.iter().any(Vec::is_empty) {
        return Err("Translation profiles need at least one non-empty tier".into());
    }
    if !(1..=1000).contains(&raw.batch_size) {
        return Err("Translation batch_size must be from 1 to 1000".into());
    }
    if raw
        .title_context
        .as_ref()
        .is_some_and(|s| s.chars().count() > 512)
    {
        return Err("Translation title_context must be at most 512 characters".into());
    }
    let mut tiers = Vec::with_capacity(raw.tiers.len());
    for tier in raw.tiers {
        let mut entries = Vec::with_capacity(tier.len());
        for entry in tier {
            let provider = entry.provider.to_ascii_lowercase();
            if !matches!(
                provider.as_str(),
                "local"
                    | "custom"
                    | "openai"
                    | "google"
                    | "gemini"
                    | "groq"
                    | "openrouter"
                    | "mistral"
                    | "github"
                    | "nvidia"
            ) {
                return Err("Unsupported translation provider".into());
            }
            if entry.model.trim().is_empty() || entry.model.chars().count() > 128 {
                return Err("Translation model must contain 1 to 128 characters".into());
            }
            let needs_key = !matches!(provider.as_str(), "local" | "custom");
            let key = api_key(entry.api_key_env, needs_key, resolve_env)?;
            if provider == "custom"
                && entry
                    .api_url
                    .as_deref()
                    .is_none_or(|url| url.trim().is_empty())
            {
                return Err("Custom translation providers require api_url".into());
            }
            if let Some(url) = entry.api_url.as_deref() {
                validate_api_url(url, "Translation api_url")?;
            }
            entries.push(TierEntry {
                provider,
                model: entry.model,
                api_key: key,
                api_url: entry.api_url,
                rpm: entry.rpm,
                max_requests: entry.max_requests,
            });
        }
        tiers.push(entries);
    }
    Ok(TranslationProfile {
        tiers,
        batch_size: raw.batch_size,
        resume_overlap: raw.resume_overlap,
        title_context: raw.title_context,
    })
}

fn resolve_transcription(
    raw: RawTranscriptionProfile,
    resolve_env: &impl Fn(&str) -> Option<String>,
) -> Result<TranscriptionProfile, String> {
    let provider = raw.provider.to_ascii_lowercase();
    if !matches!(
        provider.as_str(),
        "local"
            | "whisper"
            | "local_whisper"
            | "groq"
            | "openai"
            | "deepgram"
            | "assemblyai"
            | "custom"
            | "custom_whisper"
    ) {
        return Err("Unsupported transcription provider".into());
    }
    if raw.model.trim().is_empty() || raw.model.chars().count() > 128 {
        return Err("Transcription model must contain 1 to 128 characters".into());
    }
    let is_local = matches!(provider.as_str(), "local" | "whisper" | "local_whisper");
    let is_custom = matches!(provider.as_str(), "custom" | "custom_whisper");
    if is_local && srt_transcribe::model::validate_model_id(&raw.model).is_err() {
        return Err("Local transcription model must be a catalog Whisper model ID".into());
    }
    let api_key = api_key(raw.api_key_env, !is_local && !is_custom, resolve_env)?;
    if is_custom
        && raw
            .api_url
            .as_deref()
            .is_none_or(|url| url.trim().is_empty())
    {
        return Err("Custom transcription providers require api_url".into());
    }
    if let Some(url) = raw.api_url.as_deref() {
        validate_api_url(url, "Transcription api_url")?;
    }
    if !is_local
        && (raw.word_timestamps
            || raw.quality
            || raw.vad
            || !raw.use_gpu
            || raw.allow_model_download)
    {
        return Err(
            "word_timestamps, quality, vad, use_gpu and allow_model_download are local-provider options".into(),
        );
    }
    Ok(TranscriptionProfile {
        provider,
        model: raw.model,
        api_key,
        api_url: raw.api_url,
        translate_to_english: raw.translate_to_english,
        word_timestamps: raw.word_timestamps,
        quality: raw.quality,
        vad: raw.vad,
        use_gpu: raw.use_gpu,
        max_segment_length: raw.max_segment_length,
        allow_model_download: raw.allow_model_download,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<ProviderConfig, String> {
        ProviderConfig::parse_with(text, |name| match name {
            "TEST_KEY" => Some("top-secret-value".to_owned()),
            _ => None,
        })
    }

    #[test]
    fn local_profiles_default_enabled_and_can_build_engine_transcription_config() {
        let config = parse(
            r#"{"transcription":{"profiles":{"local":{"provider":"local","model":"base"}}}}"#,
        )
        .unwrap();
        assert!(!config.translation_available());
        assert!(config.transcription_available());
        assert_eq!(config.transcription_profile_ids(), vec!["local"]);
        let engine = config.transcription_profile("local").unwrap().config(
            "in.wav".into(),
            "out.srt".into(),
            None,
        );
        assert_eq!(engine.language, "auto");
        assert_eq!(engine.provider.as_deref(), Some("local"));
        assert!(engine.use_gpu);
    }

    #[test]
    fn resolves_translation_secrets_without_debug_or_serialization_surface() {
        let config = parse(
            r#"{"translation":{"profiles":{"default":{"tiers":[[{"provider":"google","model":"gemini-2.5-flash","api_key_env":"TEST_KEY"}]],"batch_size":12,"resume_overlap":3,"title_context":"Film"}}}}"#,
        )
        .unwrap();
        let profile = config.translation_profile("default").unwrap();
        assert_eq!(profile.batch_size, 12);
        assert_eq!(profile.resume_overlap, 3);
        assert_eq!(profile.title_context.as_deref(), Some("Film"));
        assert_eq!(
            profile.tiers[0][0].api_key.as_deref(),
            Some("top-secret-value")
        );
        assert_eq!(profile.tiers[0][0].provider, "google");
    }

    #[test]
    fn rejects_unknown_fields_invalid_providers_and_missing_credentials() {
        for text in [
            r#"{"unknown":true}"#,
            r#"{"transcription":{"profiles":{"x":{"provider":"wat","model":"base"}}}}"#,
            r#"{"transcription":{"profiles":{"x":{"provider":"groq","model":"whisper-large-v3"}}}}"#,
            r#"{"transcription":{"profiles":{"x":{"provider":"local","model":"base","api_key":"secret"}}}}"#,
        ] {
            let error = parse(text).err().unwrap();
            assert!(!error.contains("top-secret-value"));
            assert!(!error.contains("secret\""));
        }
    }

    #[test]
    fn rejects_bad_tiers_and_non_local_only_transcription_options() {
        let no_tier = parse(r#"{"translation":{"profiles":{"x":{"tiers":[]}}}}"#);
        assert!(no_tier.err().unwrap().contains("non-empty tier"));
        let ignored = parse(
            r#"{"transcription":{"profiles":{"x":{"provider":"openai","model":"whisper-1","api_key_env":"TEST_KEY","quality":true}}}}"#,
        );
        assert!(ignored.err().unwrap().contains("local-provider options"));
    }

    #[test]
    fn rejects_invalid_env_names_and_credential_bearing_urls_without_echoing_values() {
        let bad_env = parse(
            r#"{"transcription":{"profiles":{"x":{"provider":"groq","model":"whisper-large-v3","api_key_env":"9BAD"}}}}"#,
        );
        assert!(
            bad_env
                .err()
                .unwrap()
                .contains("environment variable names")
        );

        let credential_url = parse(
            r#"{"transcription":{"profiles":{"x":{"provider":"custom","model":"whisper","api_url":"https://user:secret-token@example.test/v1"}}}}"#,
        );
        let error = credential_url.err().unwrap();
        assert!(error.contains("without embedded credentials"));
        assert!(!error.contains("secret-token"));
    }

    #[test]
    fn local_transcription_models_must_be_in_catalog_and_cloud_cannot_enable_downloads() {
        let unknown_local = parse(
            r#"{"transcription":{"profiles":{"x":{"provider":"local","model":"../outside"}}}}"#,
        );
        assert!(
            unknown_local
                .err()
                .unwrap()
                .contains("catalog Whisper model ID")
        );

        let cloud_download = parse(
            r#"{"transcription":{"profiles":{"x":{"provider":"groq","model":"whisper-large-v3","api_key_env":"TEST_KEY","allow_model_download":true}}}}"#,
        );
        assert!(
            cloud_download
                .err()
                .unwrap()
                .contains("allow_model_download are local-provider options")
        );
    }

    #[test]
    fn unresolved_environment_and_profile_errors_do_not_echo_supplied_identifiers() {
        let unresolved = ProviderConfig::parse_with(
            r#"{"transcription":{"profiles":{"x":{"provider":"groq","model":"whisper","api_key_env":"TOKEN_NAME"}}}}"#,
            |_| None,
        )
        .err()
        .unwrap();
        assert!(unresolved.contains("unset or empty"));
        assert!(!unresolved.contains("TOKEN_NAME"));

        let unknown = ProviderConfig::default()
            .transcription_profile("sensitive-credential-looking-value")
            .err()
            .unwrap();
        assert!(!unknown.contains("sensitive-credential-looking-value"));
    }
}

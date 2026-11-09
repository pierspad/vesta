pub mod archive;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod url;

use archive::{
    copy_dir_recursive, fetch_and_extract_plugin, find_local_plugin_dir, load_manifest_from_dir,
};
use chrono::Utc;
use manifest::PluginManifest;
use registry::{InstalledPlugin, PluginRegistry};
use runtime::PluginRuntimeManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;
use url::{ResolvedPluginSource, resolve_plugin_url};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInspectResult {
    pub manifest: PluginManifest,
    pub source: ResolvedPluginSource,
    #[serde(default)]
    pub readme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEventPayload {
    #[serde(default)]
    pub screen: Option<String>,
    pub widget_id: String,
    pub event_type: String,
    #[serde(default)]
    pub values: serde_json::Value,
    #[serde(default)]
    pub expert_mode: Option<bool>,
}

pub struct PluginManager {
    base_dir: PathBuf,
    registry_file: PathBuf,
    registry: Arc<Mutex<PluginRegistry>>,
    runtime: PluginRuntimeManager,
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Resolves custom icon defined in manifest either via `manifest.icon` or `manifest.tab.icon`.
/// Supports direct data URIs, HTTP(S) URLs, or relative image files inside `plugin_dir` (<= 512 KB).
pub fn resolve_plugin_icon_data_url(
    plugin_dir: &std::path::Path,
    manifest: &PluginManifest,
) -> Option<String> {
    let raw = manifest
        .icon
        .as_deref()
        .or_else(|| manifest.tab.as_ref().and_then(|t| t.icon.as_deref()))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("data:image/")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
    {
        return Some(trimmed.to_string());
    }

    // Reject path traversal or absolute paths
    if trimmed.contains("..") || trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return None;
    }
    if trimmed.len() >= 2
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && trimmed.chars().nth(1) == Some(':')
    {
        return None;
    }

    let full_path = plugin_dir.join(trimmed);
    if !full_path.exists() || !full_path.is_file() {
        return None;
    }

    let canon_plugin = plugin_dir.canonicalize().ok()?;
    let canon_file = full_path.canonicalize().ok()?;
    if !canon_file.starts_with(&canon_plugin) {
        return None;
    }

    // Enforce 512 KB max file size
    let metadata = fs::metadata(&canon_file).ok()?;
    if metadata.len() > 512 * 1024 {
        return None;
    }

    let ext = canon_file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mime = match ext.as_str() {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        _ => return None,
    };

    let bytes = fs::read(&canon_file).ok()?;
    let b64 = base64_encode(&bytes);
    Some(format!("data:{mime};base64,{b64}"))
}

impl PluginManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let registry_file = base_dir.join("plugins.json");
        let registry = PluginRegistry::load_from_file(&registry_file).unwrap_or_default();
        Self {
            base_dir,
            registry_file,
            registry: Arc::new(Mutex::new(registry)),
            runtime: PluginRuntimeManager::new(),
        }
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.base_dir.join("installed")
    }

    /// Returns list of all installed plugins.
    pub async fn list(&self) -> Vec<InstalledPlugin> {
        let reg = self.registry.lock().await;
        let mut list = reg.list();
        for plugin in &mut list {
            if plugin.icon_url.is_none() {
                let plugin_dir = self.plugins_dir().join(&plugin.id);
                plugin.icon_url = resolve_plugin_icon_data_url(&plugin_dir, &plugin.manifest);
            }
        }
        list
    }

    /// Inspects a plugin repository/archive without installing it permanently,
    /// returning the manifest and permissions for user confirmation.
    pub async fn inspect(
        &self,
        url: &str,
        git_ref: Option<&str>,
    ) -> Result<PluginInspectResult, String> {
        let source = resolve_plugin_url(url, git_ref)?;

        let local_path = Path::new(&source.git_clone_url);
        if local_path.is_dir() {
            let local_dir = find_local_plugin_dir(local_path)?;
            let manifest = load_manifest_from_dir(&local_dir)?;
            let readme = manifest.readme.clone();
            let icon_url = resolve_plugin_icon_data_url(&local_dir, &manifest);

            return Ok(PluginInspectResult {
                manifest,
                source,
                readme,
                icon_url,
            });
        }

        let temp_dir = tempfile::Builder::new()
            .prefix("vesta-plugin-inspect-")
            .tempdir()
            .map_err(|e| format!("Failed to create temporary directory for inspection: {e}"))?;

        let manifest = fetch_and_extract_plugin(&source, temp_dir.path()).await?;
        let readme = manifest.readme.clone();
        let icon_url = resolve_plugin_icon_data_url(temp_dir.path(), &manifest);

        Ok(PluginInspectResult {
            manifest,
            source,
            readme,
            icon_url,
        })
    }

    /// Installs a plugin from a resolved URL or repository into `<plugins_dir>/<id>`.
    pub async fn install(
        &self,
        url: &str,
        git_ref: Option<&str>,
    ) -> Result<InstalledPlugin, String> {
        let source = resolve_plugin_url(url, git_ref)?;

        let local_path = Path::new(&source.git_clone_url);
        if local_path.is_dir() {
            find_local_plugin_dir(local_path)?;
        }

        // Download to a staging directory first
        let staging_temp = tempfile::Builder::new()
            .prefix("vesta-plugin-staging-")
            .tempdir()
            .map_err(|e| format!("Failed to create staging directory: {e}"))?;

        let manifest = fetch_and_extract_plugin(&source, staging_temp.path()).await?;

        let plugins_dir = self.plugins_dir();
        fs::create_dir_all(&plugins_dir)
            .map_err(|e| format!("Failed to create plugins directory: {e}"))?;

        let target_dir = plugins_dir.join(&manifest.id);
        if target_dir.exists() {
            // Stop any active runtime before replacing
            self.runtime.stop_plugin(&manifest.id).await;
            fs::remove_dir_all(&target_dir)
                .map_err(|e| format!("Failed to remove previous plugin directory: {e}"))?;
        }

        // Move staging to destination
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create target directory: {e}"))?;

        copy_dir_recursive(staging_temp.path(), &target_dir)?;

        let (existing_installed_at, existing_settings) = {
            let reg = self.registry.lock().await;
            if let Some(existing) = reg.get(&manifest.id) {
                (
                    Some(existing.installed_at.clone()),
                    existing.settings.clone(),
                )
            } else {
                (None, HashMap::new())
            }
        };

        let now = Utc::now().to_rfc3339();
        let (installed_at, updated_at) = match existing_installed_at {
            Some(prev) => (prev, Some(now)),
            None => (now.clone(), Some(now)),
        };

        let readme = manifest.readme.clone();
        let icon_url = resolve_plugin_icon_data_url(&target_dir, &manifest);

        let installed_plugin = InstalledPlugin {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            enabled: true,
            source_url: source.original_url.clone(),
            source_ref: if source.git_ref != "HEAD" {
                Some(source.git_ref.clone())
            } else {
                None
            },
            installed_at,
            updated_at,
            readme,
            icon_url,
            manifest,
            settings: existing_settings,
        };

        {
            let mut reg = self.registry.lock().await;
            reg.add_or_update(installed_plugin.clone());
            reg.save_to_file(&self.registry_file)?;
        }

        Ok(installed_plugin)
    }

    /// Enables a plugin.
    pub async fn enable(&self, id: &str) -> Result<(), String> {
        let mut reg = self.registry.lock().await;
        reg.set_enabled(id, true)?;
        reg.save_to_file(&self.registry_file)?;
        Ok(())
    }

    /// Disables a plugin, terminating its child process if running.
    pub async fn disable(&self, id: &str) -> Result<(), String> {
        self.runtime.stop_plugin(id).await;
        let mut reg = self.registry.lock().await;
        reg.set_enabled(id, false)?;
        reg.save_to_file(&self.registry_file)?;
        Ok(())
    }

    /// Uninstalls a plugin, stopping any active process, deleting its files and registry entry.
    pub async fn uninstall(&self, id: &str) -> Result<(), String> {
        self.runtime.stop_plugin(id).await;

        let plugin_dir = self.plugins_dir().join(id);
        if plugin_dir.exists() {
            let _ = fs::remove_dir_all(plugin_dir);
        }

        let mut reg = self.registry.lock().await;
        reg.remove(id);
        reg.save_to_file(&self.registry_file)?;
        Ok(())
    }

    /// Updates a plugin by re-fetching from its original source URL.
    pub async fn update(&self, id: &str) -> Result<InstalledPlugin, String> {
        let (source_url, source_ref) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(id)
                .ok_or_else(|| format!("Plugin '{id}' not found"))?;
            (plugin.source_url.clone(), plugin.source_ref.clone())
        };

        self.install(&source_url, source_ref.as_deref()).await
    }

    /// Gets current settings for a plugin.
    pub async fn get_settings(
        &self,
        id: &str,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let reg = self.registry.lock().await;
        let plugin = reg
            .get(id)
            .ok_or_else(|| format!("Plugin '{id}' not found"))?;
        Ok(plugin.settings.clone())
    }

    /// Saves settings for a plugin.
    pub async fn save_settings(
        &self,
        id: &str,
        settings: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let mut reg = self.registry.lock().await;
        reg.update_settings(id, settings)?;
        reg.save_to_file(&self.registry_file)?;
        Ok(())
    }

    /// Lists all shared credentials.
    pub async fn list_shared_credentials(&self) -> std::collections::BTreeMap<String, String> {
        let reg = self.registry.lock().await;
        reg.get_all_shared_credentials()
    }

    /// Sets or removes a shared credential.
    pub async fn set_shared_credential(
        &self,
        key: String,
        value: Option<String>,
    ) -> Result<(), String> {
        let mut reg = self.registry.lock().await;
        let mut updated = reg.clone();
        updated.set_shared_credential(key, value);
        updated.save_shared_credentials(&self.registry_file)?;
        reg.shared_credentials = updated.shared_credentials;
        Ok(())
    }

    async fn process_returned_shared_credentials(
        &self,
        manifest: &PluginManifest,
        res: &serde_json::Value,
    ) -> Result<(), String> {
        let set_obj = res
            .get("setSharedCredentials")
            .or_else(|| res.get("root").and_then(|r| r.get("setSharedCredentials")));
        let Some(serde_json::Value::Object(map)) = set_obj else {
            return Ok(());
        };

        let allowed = manifest.requested_shared_credentials();
        let mut reg = self.registry.lock().await;
        let mut updated = reg.clone();
        for (key, value) in map {
            if !allowed.contains(key) {
                continue;
            }
            if let Some(value) = value.as_str() {
                let value = (!value.trim().is_empty()).then(|| value.to_owned());
                updated.set_shared_credential(key.clone(), value);
            } else if value.is_null() {
                updated.set_shared_credential(key.clone(), None);
            }
        }

        if updated.shared_credentials != reg.shared_credentials {
            updated.save_shared_credentials(&self.registry_file)?;
            reg.shared_credentials = updated.shared_credentials;
        }
        Ok(())
    }

    /// Renders a screen in a plugin.
    pub async fn render(
        &self,
        app_handle: Option<AppHandle>,
        plugin_id: &str,
        screen: &str,
        params: serde_json::Value,
        locale: &str,
        expert_mode: bool,
    ) -> Result<serde_json::Value, String> {
        let (manifest, settings, shared_creds) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(plugin_id)
                .ok_or_else(|| format!("Plugin '{plugin_id}' not found"))?;
            if !plugin.enabled {
                return Err(format!("Plugin '{plugin_id}' is currently disabled"));
            }
            let allowed = plugin.manifest.requested_shared_credentials();
            let creds = reg.get_shared_credentials(&allowed);
            (plugin.manifest.clone(), plugin.settings.clone(), creds)
        };

        let plugin_dir = self.plugins_dir().join(plugin_id);
        if !plugin_dir.exists() {
            return Err(format!(
                "Plugin directory not found: {}",
                plugin_dir.display()
            ));
        }

        let process = self
            .runtime
            .ensure_running(
                &manifest,
                &plugin_dir,
                app_handle,
                locale,
                &settings,
                expert_mode,
                &shared_creds,
            )
            .await?;

        let res = self
            .runtime
            .render(&process, screen, params, locale, expert_mode, &shared_creds)
            .await?;

        self.process_returned_shared_credentials(&manifest, &res)
            .await?;
        Ok(res)
    }

    /// Dispatches an event (click, change, etc.) to the plugin.
    pub async fn dispatch_event(
        &self,
        app_handle: Option<AppHandle>,
        plugin_id: &str,
        event: &PluginEventPayload,
        locale: &str,
        expert_mode: bool,
    ) -> Result<serde_json::Value, String> {
        let (manifest, settings, shared_creds) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(plugin_id)
                .ok_or_else(|| format!("Plugin '{plugin_id}' not found"))?;
            if !plugin.enabled {
                return Err(format!("Plugin '{plugin_id}' is currently disabled"));
            }
            let allowed = plugin.manifest.requested_shared_credentials();
            let creds = reg.get_shared_credentials(&allowed);
            (plugin.manifest.clone(), plugin.settings.clone(), creds)
        };

        let plugin_dir = self.plugins_dir().join(plugin_id);
        let process = self
            .runtime
            .ensure_running(
                &manifest,
                &plugin_dir,
                app_handle,
                locale,
                &settings,
                expert_mode,
                &shared_creds,
            )
            .await?;

        let screen_name = event.screen.as_deref().unwrap_or("main");

        let res = self
            .runtime
            .dispatch_event(
                &process,
                screen_name,
                &event.widget_id,
                &event.event_type,
                event.values.clone(),
                locale,
                expert_mode,
                &shared_creds,
            )
            .await?;

        self.process_returned_shared_credentials(&manifest, &res)
            .await?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginEntry, PluginTab};

    #[tokio::test]
    async fn credential_write_failure_keeps_last_saved_value() {
        let directory = tempfile::tempdir().unwrap();
        let host = PluginManager::new(directory.path().into());
        let key = "test_api_key".to_owned();
        host.set_shared_credential(key.clone(), Some("saved".into()))
            .await
            .unwrap();

        let reloaded = PluginManager::new(directory.path().into());
        assert_eq!(reloaded.list_shared_credentials().await[&key], "saved");

        let vault = directory.path().join("shared_credentials.json");
        fs::remove_file(&vault).unwrap();
        fs::create_dir(&vault).unwrap();
        assert!(
            host.set_shared_credential(key.clone(), Some("unsaved".into()))
                .await
                .is_err()
        );
        assert_eq!(host.list_shared_credentials().await[&key], "saved");
    }

    fn make_test_manifest(icon: Option<&str>, tab_icon: Option<&str>) -> PluginManifest {
        PluginManifest {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            api_version: 1,
            description: None,
            long_description: None,
            readme: None,
            icon: icon.map(|s| s.to_string()),
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec!["python3".to_string(), "main.py".to_string()],
                os: None,
            },
            permissions: crate::plugins::manifest::PluginPermissions::default(),
            shared_credentials: vec![],
            tab: tab_icon.map(|ti| PluginTab {
                title: "Test".to_string(),
                icon: Some(ti.to_string()),
            }),
            settings: None,
        }
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn test_resolve_remote_and_data_icons() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = make_test_manifest(Some("https://example.com/logo.svg"), None);
        assert_eq!(
            resolve_plugin_icon_data_url(temp.path(), &manifest),
            Some("https://example.com/logo.svg".to_string())
        );

        let data_url = "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=";
        let manifest_data = make_test_manifest(Some(data_url), None);
        assert_eq!(
            resolve_plugin_icon_data_url(temp.path(), &manifest_data),
            Some(data_url.to_string())
        );
    }

    #[test]
    fn test_resolve_local_image_files() {
        let temp = tempfile::tempdir().unwrap();
        let svg_path = temp.path().join("icon.svg");
        fs::write(&svg_path, "<svg></svg>").unwrap();

        let manifest = make_test_manifest(Some("icon.svg"), None);
        let resolved = resolve_plugin_icon_data_url(temp.path(), &manifest).unwrap();
        assert!(resolved.starts_with("data:image/svg+xml;base64,"));

        // Fallback to tab.icon if manifest.icon is not set
        let manifest_tab = make_test_manifest(None, Some("icon.svg"));
        let resolved_tab = resolve_plugin_icon_data_url(temp.path(), &manifest_tab).unwrap();
        assert!(resolved_tab.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn test_resolve_named_icon_fallback() {
        let temp = tempfile::tempdir().unwrap();
        // Named icons like "puzzle" or "film" are not image files so they resolve to None
        let manifest = make_test_manifest(None, Some("puzzle"));
        assert_eq!(resolve_plugin_icon_data_url(temp.path(), &manifest), None);
    }

    #[test]
    fn test_resolve_traversal_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let manifest = make_test_manifest(Some("../secret.png"), None);
        assert_eq!(resolve_plugin_icon_data_url(temp.path(), &manifest), None);
    }

    #[tokio::test]
    async fn test_inspect_local_plugin_direct() {
        let base = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(base.path().to_path_buf());

        let plugin_dir = tempfile::tempdir().unwrap();
        let manifest_content = r#"{
            "id": "direct-inspect-plugin",
            "name": "Direct Inspect Plugin",
            "version": "1.0.0",
            "apiVersion": 1,
            "entry": { "command": ["python3", "main.py"] }
        }"#;
        fs::write(
            plugin_dir.path().join("vesta-plugin.json"),
            manifest_content,
        )
        .unwrap();

        let inspect_res = pm
            .inspect(plugin_dir.path().to_str().unwrap(), None)
            .await
            .expect("direct local inspect failed");

        assert_eq!(inspect_res.manifest.id, "direct-inspect-plugin");
        assert_eq!(inspect_res.manifest.name, "Direct Inspect Plugin");
    }

    #[tokio::test]
    async fn test_inspect_local_folder_missing_manifest_fails_instantly() {
        let base = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(base.path().to_path_buf());

        let empty_dir = tempfile::tempdir().unwrap();
        let err = pm
            .inspect(empty_dir.path().to_str().unwrap(), None)
            .await
            .unwrap_err();

        assert_eq!(
            err,
            format!(
                "No vesta-plugin.json manifest found in '{}'",
                empty_dir.path().display()
            )
        );
    }

    #[tokio::test]
    async fn test_install_local_folder_missing_manifest_fails_instantly() {
        let base = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(base.path().to_path_buf());

        let empty_dir = tempfile::tempdir().unwrap();
        let err = pm
            .install(empty_dir.path().to_str().unwrap(), None)
            .await
            .unwrap_err();

        assert_eq!(
            err,
            format!(
                "No vesta-plugin.json manifest found in '{}'",
                empty_dir.path().display()
            )
        );
    }
}

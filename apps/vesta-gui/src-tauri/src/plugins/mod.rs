pub mod archive;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod url;

use archive::{fetch_and_extract_plugin, find_local_plugin_dir, load_manifest_from_dir};
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
        self.install_source(url, git_ref, None).await
    }

    async fn install_source(
        &self,
        url: &str,
        git_ref: Option<&str>,
        update_id: Option<&str>,
    ) -> Result<InstalledPlugin, String> {
        let source = resolve_plugin_url(url, git_ref)?;

        let local_path = Path::new(&source.git_clone_url);
        if local_path.is_dir() {
            find_local_plugin_dir(local_path)?;
        }

        let plugins_dir = self.plugins_dir();
        fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;
        let staging = tempfile::tempdir_in(&plugins_dir).map_err(|e| e.to_string())?;
        let incoming = staging.path().join("incoming");
        fs::create_dir(&incoming).map_err(|e| e.to_string())?;
        let manifest = fetch_and_extract_plugin(&source, &incoming).await?;
        if update_id.is_some_and(|id| id != manifest.id) {
            return Err("Update manifest does not match the installed plugin ID".into());
        }

        if update_id.is_some()
            && let Some(tag) = git_ref
            && tag.starts_with('v')
            && tag[1..].split('.').count() == 3
            && manifest.version.trim_start_matches('v') != &tag[1..]
        {
            return Err("Plugin release tag and manifest version do not match".into());
        }
        let mut reg = self.registry.lock().await;
        let existing = reg.get(&manifest.id);
        if existing.is_some_and(|plugin| plugin.source_url != source.original_url) {
            return Err("A plugin with this ID is installed from a different source".into());
        }
        if update_id.is_some() && existing.is_none() {
            return Err("Plugin was uninstalled while checking for an update".into());
        }
        if update_id.is_some()
            && existing.is_some_and(|plugin| {
                manifest
                    .permissions
                    .scopes
                    .iter()
                    .any(|scope| !plugin.manifest.permissions.scopes.contains(scope))
            })
        {
            return Err(
                "Update requests additional permissions. Inspect and install it explicitly.".into(),
            );
        }
        let existing_installed_at = existing.map(|plugin| plugin.installed_at.clone());
        let existing_settings = existing
            .map(|plugin| plugin.settings.clone())
            .unwrap_or_default();
        let enabled = existing.is_none_or(|plugin| plugin.enabled);
        let target_dir = plugins_dir.join(&manifest.id);
        let backup = staging.path().join("previous");
        let had_previous = target_dir.exists();
        let data = target_dir.join("data");
        if had_previous && incoming.join("data").exists() {
            return Err("Plugin update archive must not contain a data directory".into());
        }
        if had_previous {
            self.runtime.stop_plugin(&manifest.id).await;
            fs::rename(&target_dir, &backup).map_err(|e| e.to_string())?;
        }
        let publish = (|| -> Result<(), String> {
            fs::rename(&incoming, &target_dir).map_err(|e| e.to_string())?;
            if backup.join("data").exists() {
                fs::rename(backup.join("data"), &data).map_err(|e| e.to_string())?;
            }
            Ok(())
        })();

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
            enabled,
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

        let mut updated = reg.clone();
        updated.add_or_update(installed_plugin.clone());
        if let Err(error) = publish.and_then(|()| updated.save_to_file(&self.registry_file)) {
            let restore = (|| -> std::io::Result<()> {
                if had_previous && data.exists() {
                    fs::rename(&data, backup.join("data"))?;
                }
                if target_dir.exists() {
                    fs::rename(&target_dir, &incoming)?;
                }
                if had_previous {
                    fs::rename(&backup, &target_dir)?;
                }
                Ok(())
            })();
            if let Err(restore_error) = restore {
                let recovery = staging.keep();
                return Err(format!(
                    "{error}; restore failed: {restore_error}. Recovery files: {}",
                    recovery.display()
                ));
            }
            return Err(error);
        }
        *reg = updated;

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
        if !is_valid_plugin_id(id) {
            return Err(format!("Invalid plugin id '{id}'"));
        }

        {
            let reg = self.registry.lock().await;
            if reg.get(id).is_none() {
                return Err(format!("Plugin '{id}' is not installed"));
            }
        }

        self.runtime.stop_plugin(id).await;

        let plugins_root = self.plugins_dir();
        let plugin_dir = plugins_root.join(id);
        if fs::symlink_metadata(&plugin_dir).is_ok() {
            let canon_root = fs::canonicalize(&plugins_root)
                .map_err(|e| format!("Failed to resolve plugins directory: {e}"))?;
            let canon_dir = fs::canonicalize(&plugin_dir)
                .map_err(|e| format!("Failed to resolve plugin directory: {e}"))?;
            if !canon_dir.starts_with(&canon_root) || canon_dir == canon_root {
                return Err(format!(
                    "Refusing to delete path outside plugins directory for '{id}'"
                ));
            }
            fs::remove_dir_all(&canon_dir)
                .map_err(|e| format!("Failed to delete plugin files: {e}"))?;
        }

        let mut reg = self.registry.lock().await;
        reg.remove(id);
        reg.save_to_file(&self.registry_file)?;
        Ok(())
    }

    /// Updates a plugin by re-fetching from its original source URL.
    pub async fn update(
        &self,
        id: &str,
        release_tag: Option<&str>,
    ) -> Result<InstalledPlugin, String> {
        if let Some(tag) = release_tag {
            let parts: Vec<_> = tag.strip_prefix('v').unwrap_or("").split('.').collect();
            if parts.len() != 3
                || parts.iter().any(|part| {
                    part.parse::<u64>().is_err() || !part.bytes().all(|byte| byte.is_ascii_digit())
                })
            {
                return Err("Invalid plugin release tag".into());
            }
        }
        let (source_url, source_ref) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(id)
                .ok_or_else(|| format!("Plugin '{id}' not found"))?;
            (plugin.source_url.clone(), plugin.source_ref.clone())
        };

        self.install_source(&source_url, release_tag.or(source_ref.as_deref()), Some(id))
            .await
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

    /// Saves settings for a plugin and stops any running process so the next render reloads them.
    pub async fn save_settings(
        &self,
        id: &str,
        settings: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        {
            let mut reg = self.registry.lock().await;
            reg.update_settings(id, settings)?;
            reg.save_to_file(&self.registry_file)?;
        }
        self.runtime.stop_plugin(id).await;
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
        let (manifest, settings) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(plugin_id)
                .ok_or_else(|| format!("Plugin '{plugin_id}' not found"))?;
            if !plugin.enabled {
                return Err(format!("Plugin '{plugin_id}' is currently disabled"));
            }
            (plugin.manifest.clone(), plugin.settings.clone())
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
            )
            .await?;

        let res = self
            .runtime
            .render(&process, screen, params, locale, expert_mode)
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
        let (manifest, settings) = {
            let reg = self.registry.lock().await;
            let plugin = reg
                .get(plugin_id)
                .ok_or_else(|| format!("Plugin '{plugin_id}' not found"))?;
            if !plugin.enabled {
                return Err(format!("Plugin '{plugin_id}' is currently disabled"));
            }
            (plugin.manifest.clone(), plugin.settings.clone())
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
            )
            .await?;

        Ok(res)
    }
}

fn is_local_fs_path(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("blob:")
        || lower.starts_with("data:")
        || lower.starts_with("stream:")
    {
        return false;
    }
    if trimmed.starts_with('/') || trimmed.starts_with('~') {
        return true;
    }
    let bytes = trimmed.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn resolve_local_media_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    if trimmed == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(trimmed)
}

fn allowlist_media_field(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    allowlist: &crate::media_stream::MediaAllowlist,
) {
    let Some(val) = obj.get_mut(key) else {
        return;
    };
    let Some(raw) = val.as_str() else {
        return;
    };
    if !is_local_fs_path(raw) {
        return;
    }
    let resolved = resolve_local_media_path(raw);
    if allowlist.allow_file(&resolved).is_ok() && raw.trim().starts_with('~') {
        *val = serde_json::Value::String(resolved.to_string_lossy().into_owned());
    }
}

fn allowlist_widget_tree(
    node: &mut serde_json::Value,
    allowlist: &crate::media_stream::MediaAllowlist,
) {
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    if obj.get("type").and_then(|v| v.as_str()) == Some("mediaPlayer") {
        allowlist_media_field(obj, "src", allowlist);
        allowlist_media_field(obj, "poster", allowlist);
    }
    if let Some(serde_json::Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            allowlist_widget_tree(child, allowlist);
        }
    }
}

/// Registers local filesystem paths referenced by `mediaPlayer` widgets in `MediaAllowlist`.
pub fn allowlist_plugin_media_paths(
    response: &mut serde_json::Value,
    allowlist: &crate::media_stream::MediaAllowlist,
) {
    let Some(obj) = response.as_object_mut() else {
        return;
    };
    if let Some(root) = obj.get_mut("root") {
        allowlist_widget_tree(root, allowlist);
    }
    if let Some(serde_json::Value::Object(footer)) = obj.get_mut("footer") {
        for slot in ["left", "center", "right"] {
            if let Some(serde_json::Value::Array(nodes)) = footer.get_mut(slot) {
                for node in nodes {
                    allowlist_widget_tree(node, allowlist);
                }
            }
        }
    }
    if let Some(serde_json::Value::Object(patch)) = obj.get_mut("patch") {
        if let Some(serde_json::Value::Object(insert)) = patch.get_mut("insert")
            && let Some(node) = insert.get_mut("node")
        {
            allowlist_widget_tree(node, allowlist);
        }
        if let Some(serde_json::Value::Object(set_props)) = patch.get_mut("setProps") {
            for props in set_props.values_mut() {
                if let Some(props_obj) = props.as_object_mut() {
                    allowlist_media_field(props_obj, "src", allowlist);
                    allowlist_media_field(props_obj, "poster", allowlist);
                }
            }
        }
    }
}

fn is_valid_plugin_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginEntry, PluginTab};

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
            tab: tab_icon.map(|ti| PluginTab {
                title: "Test".to_string(),
                icon: Some(ti.to_string()),
                settings_screen: None,
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
    async fn local_update_preserves_data_and_rejects_changed_id() {
        let base = tempfile::tempdir().unwrap();
        let manager = PluginManager::new(base.path().join("plugins"));
        let source = base.path().join("source");
        fs::create_dir(&source).unwrap();
        let path = source.to_str().unwrap();
        let mut manifest = make_test_manifest(None, None);
        let manifest_path = source.join("vesta-plugin.json");
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        manager.install(path, None).await.unwrap();

        let data = manager.plugins_dir().join(&manifest.id).join("data");
        fs::create_dir(&data).unwrap();
        fs::write(data.join("settings.json"), b"retained").unwrap();
        manager.disable(&manifest.id).await.unwrap();
        manifest.version = "2.0.0".into();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let updated = manager.update(&manifest.id, None).await.unwrap();
        assert!(!updated.enabled);
        assert_eq!(updated.version, "2.0.0");
        assert_eq!(fs::read(data.join("settings.json")).unwrap(), b"retained");

        manifest.id = "different-id".into();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(manager.update(&updated.id, None).await.is_err());
        assert!(data.join("settings.json").is_file());
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

    #[tokio::test]
    async fn test_uninstall_rejects_traversal_and_unregistered_ids() {
        let base = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(base.path().to_path_buf());
        let sentinel = base.path().join("sentinel.txt");
        fs::write(&sentinel, "keep").unwrap();

        for bad_id in ["..", "../escape", "nested/id", ""] {
            let err = pm.uninstall(bad_id).await.unwrap_err();
            assert!(err.contains("Invalid plugin id"), "id={bad_id}, err={err}");
        }
        let err = pm.uninstall("not-installed-plugin").await.unwrap_err();
        assert!(err.contains("is not installed"), "err={err}");
        assert!(sentinel.exists(), "sentinel file must remain untouched");
    }

    #[tokio::test]
    async fn save_settings_stops_running_plugin_and_reinitializes_with_new_settings() {
        let base = tempfile::tempdir().unwrap();
        let pm = PluginManager::new(base.path().join("plugins"));
        let source = base.path().join("source");
        fs::create_dir(&source).unwrap();

        let script = r#"
import json, sys
settings = {}
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        settings = req.get("params", {}).get("settings", {})
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}), flush=True)
    elif m == "render":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"root": {"type": "text", "text": str(settings.get("apiKey", ""))}}}), flush=True)
    elif m == "shutdown":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": None}), flush=True)
        break
"#;
        let manifest = serde_json::json!({
            "id": "settings-reload-plugin",
            "name": "Settings Reload Plugin",
            "version": "1.0.0",
            "apiVersion": 1,
            "entry": { "command": ["python3", "-u", "-c", script] }
        });
        fs::write(
            source.join("vesta-plugin.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        pm.install(source.to_str().unwrap(), None).await.unwrap();

        let mut initial_settings = HashMap::new();
        initial_settings.insert("apiKey".to_string(), serde_json::json!("first-key"));
        pm.save_settings("settings-reload-plugin", initial_settings)
            .await
            .unwrap();

        let first_render = pm
            .render(
                None,
                "settings-reload-plugin",
                "main",
                serde_json::json!({}),
                "en",
                false,
            )
            .await
            .unwrap();
        assert_eq!(first_render["root"]["text"], "first-key");

        let mut updated_settings = HashMap::new();
        updated_settings.insert("apiKey".to_string(), serde_json::json!("second-key"));
        pm.save_settings("settings-reload-plugin", updated_settings)
            .await
            .unwrap();

        let second_render = pm
            .render(
                None,
                "settings-reload-plugin",
                "main",
                serde_json::json!({}),
                "en",
                false,
            )
            .await
            .unwrap();
        assert_eq!(second_render["root"]["text"], "second-key");

        pm.runtime.stop_plugin("settings-reload-plugin").await;
    }

    #[test]
    fn allowlist_plugin_media_paths_registers_local_media_player_sources() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        let poster = dir.path().join("thumb.jpg");
        let patched = dir.path().join("patched.mp3");
        fs::write(&video, b"mp4").unwrap();
        fs::write(&poster, b"jpg").unwrap();
        fs::write(&patched, b"mp3").unwrap();

        let allowlist = crate::media_stream::MediaAllowlist::default();
        let mut response = serde_json::json!({
            "root": {
                "type": "card",
                "children": [
                    {
                        "type": "mediaPlayer",
                        "src": video.to_string_lossy(),
                        "poster": poster.to_string_lossy()
                    },
                    {
                        "type": "mediaPlayer",
                        "src": "https://example.com/stream.mp4"
                    }
                ]
            },
            "patch": {
                "setProps": {
                    "player_1": { "src": patched.to_string_lossy() }
                }
            }
        });

        allowlist_plugin_media_paths(&mut response, &allowlist);
        assert!(allowlist.is_allowed(&video));
        assert!(allowlist.is_allowed(&poster));
        assert!(allowlist.is_allowed(&patched));
    }
}

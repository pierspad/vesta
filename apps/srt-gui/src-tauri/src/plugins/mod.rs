pub mod archive;
pub mod manifest;
pub mod registry;
pub mod runtime;
pub mod url;

use archive::{copy_dir_recursive, fetch_and_extract_plugin};
use chrono::Utc;
use manifest::PluginManifest;
use registry::{InstalledPlugin, PluginRegistry};
use runtime::PluginRuntimeManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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
        reg.list()
    }

    /// Inspects a plugin repository/archive without installing it permanently,
    /// returning the manifest and permissions for user confirmation.
    pub async fn inspect(
        &self,
        url: &str,
        git_ref: Option<&str>,
    ) -> Result<PluginInspectResult, String> {
        let source = resolve_plugin_url(url, git_ref)?;

        let temp_dir = tempfile::Builder::new()
            .prefix("vesta-plugin-inspect-")
            .tempdir()
            .map_err(|e| format!("Failed to create temporary directory for inspection: {e}"))?;

        let manifest = fetch_and_extract_plugin(&source, temp_dir.path()).await?;
        let readme = manifest.readme.clone();

        Ok(PluginInspectResult {
            manifest,
            source,
            readme,
        })
    }

    /// Installs a plugin from a resolved URL or repository into `<plugins_dir>/<id>`.
    pub async fn install(
        &self,
        url: &str,
        git_ref: Option<&str>,
    ) -> Result<InstalledPlugin, String> {
        let source = resolve_plugin_url(url, git_ref)?;

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
            .ensure_running(&manifest, &plugin_dir, app_handle, locale, &settings, expert_mode)
            .await?;

        self.runtime.render(&process, screen, params, expert_mode).await
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
            .ensure_running(&manifest, &plugin_dir, app_handle, locale, &settings, expert_mode)
            .await?;

        let screen_name = event.screen.as_deref().unwrap_or("main");

        self.runtime
            .dispatch_event(
                &process,
                screen_name,
                &event.widget_id,
                &event.event_type,
                event.values.clone(),
                expert_mode,
            )
            .await
    }
}

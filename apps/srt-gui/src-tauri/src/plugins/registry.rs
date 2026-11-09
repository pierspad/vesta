use crate::plugins::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub enabled: bool,
    pub source_url: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    pub installed_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub readme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub manifest: PluginManifest,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistry {
    pub version: u32,
    pub plugins: Vec<InstalledPlugin>,
    #[serde(default, skip_serializing)]
    pub shared_credentials: BTreeMap<String, String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self {
            version: 1,
            plugins: Vec::new(),
            shared_credentials: BTreeMap::new(),
        }
    }
}

impl PluginRegistry {
    /// Loads the registry from the specified file path, or returns default if not found.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let parent = path.parent().unwrap_or(Path::new("."));
        let shared_cred_file = parent.join("shared_credentials.json");

        let mut registry = if !path.exists() {
            Self::default()
        } else {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read plugins registry: {e}"))?;

            let parsed: Self = serde_json::from_str(&content).map_err(|e| {
                format!(
                    "Failed to parse plugins registry from {}: {e}",
                    path.display()
                )
            })?;
            parsed
        };

        if shared_cred_file.exists()
            && let Ok(content) = fs::read_to_string(&shared_cred_file)
            && let Ok(map) = serde_json::from_str::<BTreeMap<String, String>>(&content)
        {
            registry.shared_credentials = map;
        }

        Ok(registry)
    }

    /// Atomically writes the registry and shared credentials to disk.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create registry directory: {e}"))?;

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize plugins registry: {e}"))?;

        // Write to temporary file in the same directory then rename atomically
        let temp_file = tempfile::Builder::new()
            .prefix(".plugins-")
            .tempfile_in(parent)
            .map_err(|e| format!("Failed to create temporary file for registry: {e}"))?;

        fs::write(temp_file.path(), content)
            .map_err(|e| format!("Failed to write plugins registry: {e}"))?;

        // Preserve legacy inline credentials until their separate store is durable.
        self.save_shared_credentials(path)?;
        temp_file.persist(path).map_err(|e| {
            format!(
                "Failed to persist plugins registry to {}: {e}",
                path.display()
            )
        })?;

        Ok(())
    }

    pub fn save_shared_credentials(&self, registry_path: &Path) -> Result<(), String> {
        let parent = registry_path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create credential directory: {e}"))?;
        // Save shared credentials to shared_credentials.json
        let shared_cred_file = parent.join("shared_credentials.json");
        let cred_content = serde_json::to_string_pretty(&self.shared_credentials)
            .map_err(|e| format!("Failed to serialize shared credentials: {e}"))?;
        let temp_cred = tempfile::Builder::new()
            .prefix(".shared_credentials-")
            .tempfile_in(parent)
            .map_err(|e| format!("Failed to create temporary file for shared credentials: {e}"))?;
        fs::write(temp_cred.path(), cred_content)
            .map_err(|e| format!("Failed to write shared credentials: {e}"))?;
        temp_cred.persist(&shared_cred_file).map_err(|e| {
            format!(
                "Failed to persist shared credentials to {}: {e}",
                shared_cred_file.display()
            )
        })?;

        Ok(())
    }

    pub fn list(&self) -> Vec<InstalledPlugin> {
        self.plugins.clone()
    }

    pub fn get(&self, id: &str) -> Option<&InstalledPlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut InstalledPlugin> {
        self.plugins.iter_mut().find(|p| p.id == id)
    }

    pub fn add_or_update(&mut self, plugin: InstalledPlugin) {
        if let Some(pos) = self.plugins.iter().position(|p| p.id == plugin.id) {
            // Preserve existing settings if updated plugin didn't specify new ones
            let mut updated = plugin;
            if updated.settings.is_empty() && !self.plugins[pos].settings.is_empty() {
                updated.settings = self.plugins[pos].settings.clone();
            }
            self.plugins[pos] = updated;
        } else {
            self.plugins.push(plugin);
        }
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let plugin = self
            .get_mut(id)
            .ok_or_else(|| format!("Plugin '{id}' not found in registry"))?;
        plugin.enabled = enabled;
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Option<InstalledPlugin> {
        if let Some(pos) = self.plugins.iter().position(|p| p.id == id) {
            Some(self.plugins.remove(pos))
        } else {
            None
        }
    }

    pub fn update_settings(
        &mut self,
        id: &str,
        settings: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let plugin = self
            .get_mut(id)
            .ok_or_else(|| format!("Plugin '{id}' not found in registry"))?;
        plugin.settings = settings;
        Ok(())
    }

    pub fn get_shared_credentials(&self, allowed_keys: &[String]) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();
        for key in allowed_keys {
            if let Some(val) = self.shared_credentials.get(key) {
                result.insert(key.clone(), val.clone());
            }
        }
        result
    }

    pub fn set_shared_credential(&mut self, key: String, value: Option<String>) {
        match value {
            Some(v) if !v.trim().is_empty() => {
                self.shared_credentials.insert(key, v);
            }
            _ => {
                self.shared_credentials.remove(&key);
            }
        }
    }

    pub fn get_all_shared_credentials(&self) -> BTreeMap<String, String> {
        self.shared_credentials.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginEntry, PluginPermissions};

    fn sample_manifest(id: &str) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            name: format!("Plugin {id}"),
            version: "1.0.0".to_string(),
            api_version: 1,
            description: None,
            long_description: None,
            readme: None,
            icon: None,
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec!["python3".to_string(), "main.py".to_string()],
                os: None,
            },
            permissions: PluginPermissions {
                scopes: vec!["network".to_string()],
                shared_credentials: vec!["tmdb_api_key".to_string()],
            },
            shared_credentials: vec![],
            tab: None,
            settings: None,
        }
    }

    #[test]
    fn failed_vault_write_preserves_legacy_registry_credentials() {
        let directory = tempfile::tempdir().unwrap();
        let registry_path = directory.path().join("plugins.json");
        let legacy = r#"{"version":1,"plugins":[],"sharedCredentials":{"test_key":"legacy"}}"#;
        fs::write(&registry_path, legacy).unwrap();
        let registry = PluginRegistry::load_from_file(&registry_path).unwrap();
        fs::create_dir(directory.path().join("shared_credentials.json")).unwrap();

        assert!(registry.save_to_file(&registry_path).is_err());
        assert_eq!(fs::read_to_string(registry_path).unwrap(), legacy);
    }

    #[test]
    fn test_registry_operations() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plugins.json");

        let mut reg = PluginRegistry::load_from_file(&path).unwrap();
        assert_eq!(reg.plugins.len(), 0);

        let p1 = InstalledPlugin {
            id: "p1".to_string(),
            name: "Plugin 1".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            enabled: true,
            source_url: "https://github.com/foo/p1".to_string(),
            source_ref: None,
            installed_at: "2026-10-30T12:00:00Z".to_string(),
            updated_at: None,
            readme: None,
            icon_url: None,
            manifest: sample_manifest("p1"),
            settings: HashMap::new(),
        };

        reg.add_or_update(p1.clone());
        assert_eq!(reg.list().len(), 1);

        reg.set_shared_credential("tmdb_api_key".to_string(), Some("tmdb_val".to_string()));
        reg.set_shared_credential(
            "unrelated_key".to_string(),
            Some("unrelated_val".to_string()),
        );

        reg.save_to_file(&path).unwrap();

        let mut loaded = PluginRegistry::load_from_file(&path).unwrap();
        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(loaded.get("p1").unwrap().name, "Plugin 1");

        // Verify shared credentials filtering and persistence
        let allowed = vec!["tmdb_api_key".to_string()];
        let filtered = loaded.get_shared_credentials(&allowed);
        assert_eq!(
            filtered.get("tmdb_api_key").map(|s| s.as_str()),
            Some("tmdb_val")
        );
        assert_eq!(filtered.get("unrelated_key"), None);

        // Verify deletion
        loaded.set_shared_credential("tmdb_api_key".to_string(), None);
        assert!(loaded.get_shared_credentials(&allowed).is_empty());

        loaded.set_enabled("p1", false).unwrap();
        assert!(!loaded.get("p1").unwrap().enabled);

        let mut settings = HashMap::new();
        settings.insert("apiKey".to_string(), serde_json::json!("secret123"));
        loaded.update_settings("p1", settings).unwrap();
        assert_eq!(
            loaded.get("p1").unwrap().settings.get("apiKey").unwrap(),
            "secret123"
        );

        let removed = loaded.remove("p1");
        assert!(removed.is_some());
        assert_eq!(loaded.list().len(), 0);
    }
}

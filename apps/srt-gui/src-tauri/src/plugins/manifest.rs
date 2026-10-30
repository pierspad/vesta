use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest definition for a Vesta plugin (`vesta-plugin.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub entry: PluginEntry,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub tab: Option<PluginTab>,
    #[serde(default)]
    pub settings: Option<Vec<PluginSettingField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub command: Vec<String>,
    #[serde(default)]
    pub os: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginTab {
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingField {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
}

impl PluginManifest {
    /// Validates the manifest fields and ensures security constraints.
    pub fn validate(&self) -> Result<(), String> {
        // Validate ID: alphanumeric, hyphen, underscore only. No path separators or dots.
        if self.id.is_empty() {
            return Err("Plugin ID cannot be empty".to_string());
        }
        if self.id.len() > 64 {
            return Err("Plugin ID must be 64 characters or fewer".to_string());
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(format!(
                "Invalid plugin ID '{}': only alphanumeric characters, hyphens, and underscores are allowed",
                self.id
            ));
        }

        // Validate name
        if self.name.trim().is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }
        if self.name.len() > 100 {
            return Err("Plugin name must be 100 characters or fewer".to_string());
        }

        // Validate version
        if self.version.trim().is_empty() {
            return Err("Plugin version cannot be empty".to_string());
        }

        // Validate API version
        if self.api_version != 1 {
            return Err(format!(
                "Unsupported plugin apiVersion {}. Only apiVersion 1 is supported in this Vesta release.",
                self.api_version
            ));
        }

        // Validate entry command
        if self.entry.command.is_empty() {
            return Err("Plugin entry command must specify at least an executable".to_string());
        }

        // Validate tab title if tab is declared
        if let Some(tab) = &self.tab
            && tab.title.trim().is_empty()
        {
            return Err("Plugin tab title cannot be empty".to_string());
        }

        Ok(())
    }

    /// Resolves the command to execute for the current operating system.
    pub fn resolve_command(&self) -> Vec<String> {
        let os_key = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };

        if let Some(os_map) = &self.entry.os
            && let Some(cmd) = os_map.get(os_key)
            && !cmd.is_empty()
        {
            return cmd.clone();
        }

        self.entry.command.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_manifest() {
        let json = r#"{
            "id": "my-plugin_1",
            "name": "My Plugin",
            "version": "0.1.0",
            "apiVersion": 1,
            "description": "A test plugin",
            "entry": {
                "command": ["python3", "main.py"],
                "os": {
                    "windows": ["python.exe", "main.py"]
                }
            },
            "permissions": ["network", "filesystem:downloads"],
            "tab": {
                "title": "Test Tab",
                "icon": "film"
            }
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(manifest.id, "my-plugin_1");
        assert_eq!(manifest.api_version, 1);
        assert_eq!(manifest.permissions.len(), 2);
        assert!(manifest.validate().is_ok());

        #[cfg(target_os = "windows")]
        assert_eq!(manifest.resolve_command(), vec!["python.exe", "main.py"]);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(manifest.resolve_command(), vec!["python3", "main.py"]);
    }

    #[test]
    fn test_invalid_id() {
        let mut manifest = PluginManifest {
            id: "../escape".to_string(),
            name: "Bad".to_string(),
            version: "1.0.0".to_string(),
            api_version: 1,
            description: None,
            author: None,
            entry: PluginEntry {
                command: vec!["echo".to_string()],
                os: None,
            },
            permissions: vec![],
            tab: None,
            settings: None,
        };
        assert!(manifest.validate().is_err());

        manifest.id = "bad id with spaces".to_string();
        assert!(manifest.validate().is_err());

        manifest.id = "bad/slash".to_string();
        assert!(manifest.validate().is_err());

        manifest.id = "".to_string();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_invalid_api_version() {
        let manifest = PluginManifest {
            id: "good-id".to_string(),
            name: "Good".to_string(),
            version: "1.0.0".to_string(),
            api_version: 2,
            description: None,
            author: None,
            entry: PluginEntry {
                command: vec!["echo".to_string()],
                os: None,
            },
            permissions: vec![],
            tab: None,
            settings: None,
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_empty_command() {
        let manifest = PluginManifest {
            id: "good-id".to_string(),
            name: "Good".to_string(),
            version: "1.0.0".to_string(),
            api_version: 1,
            description: None,
            author: None,
            entry: PluginEntry {
                command: vec![],
                os: None,
            },
            permissions: vec![],
            tab: None,
            settings: None,
        };
        assert!(manifest.validate().is_err());
    }
}

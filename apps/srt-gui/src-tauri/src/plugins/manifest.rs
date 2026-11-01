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
    pub long_description: Option<String>,
    #[serde(default)]
    pub readme: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    pub entry: PluginEntry,
    #[serde(default)]
    pub permissions: PluginPermissions,
    #[serde(default, rename = "sharedCredentials")]
    pub shared_credentials: Vec<SharedCredentialSpec>,
    #[serde(default)]
    pub tab: Option<PluginTab>,
    #[serde(default)]
    pub settings: Option<Vec<PluginSettingField>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginPermissions {
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default, rename = "sharedCredentials")]
    pub shared_credentials: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawPermissions {
    List(Vec<String>),
    Object {
        #[serde(default)]
        scopes: Vec<String>,
        #[serde(default, rename = "sharedCredentials")]
        shared_credentials: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for PluginPermissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawPermissions::deserialize(deserializer)?;
        match raw {
            RawPermissions::List(list) => {
                let mut scopes = Vec::new();
                let mut shared_credentials = Vec::new();
                for item in list {
                    if let Some(rest) = item
                        .strip_prefix("sharedCredential:")
                        .or_else(|| item.strip_prefix("shared_credential:"))
                        .or_else(|| item.strip_prefix("sharedCredentials:"))
                        .or_else(|| item.strip_prefix("shared:"))
                    {
                        shared_credentials.push(rest.trim().to_string());
                    } else {
                        scopes.push(item);
                    }
                }
                Ok(PluginPermissions {
                    scopes,
                    shared_credentials,
                })
            }
            RawPermissions::Object {
                scopes,
                shared_credentials,
            } => Ok(PluginPermissions {
                scopes,
                shared_credentials,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedCredentialSpec {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawSharedCredentialSpec {
    Id(String),
    Object {
        id: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        description: Option<String>,
    },
}

impl<'de> Deserialize<'de> for SharedCredentialSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawSharedCredentialSpec::deserialize(deserializer)? {
            RawSharedCredentialSpec::Id(id) => Ok(SharedCredentialSpec {
                id,
                label: None,
                description: None,
            }),
            RawSharedCredentialSpec::Object {
                id,
                label,
                description,
            } => Ok(SharedCredentialSpec {
                id,
                label,
                description,
            }),
        }
    }
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

fn validate_icon_path(icon: &str) -> Result<(), String> {
    let trimmed = icon.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if trimmed.starts_with("data:image/")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
    {
        return Ok(());
    }
    if trimmed.contains("..") || trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(format!(
            "Icon path '{icon}' cannot contain '..' or start with '/' or '\\'"
        ));
    }
    if trimmed.len() >= 2
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
        && trimmed.chars().nth(1) == Some(':')
    {
        return Err(format!("Icon path '{icon}' cannot be an absolute path"));
    }
    Ok(())
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

        // Validate custom icon paths if declared
        if let Some(icon) = &self.icon {
            validate_icon_path(icon)?;
        }
        if let Some(tab) = &self.tab
            && let Some(icon) = &tab.icon
        {
            validate_icon_path(icon)?;
        }

        // Validate shared credential IDs
        for key in self.requested_shared_credentials() {
            if key.is_empty() {
                return Err("Shared credential ID cannot be empty".to_string());
            }
            if key.len() > 64 {
                return Err("Shared credential ID must be 64 characters or fewer".to_string());
            }
            if !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(format!(
                    "Invalid shared credential ID '{key}': only alphanumeric characters, hyphens, and underscores are allowed"
                ));
            }
        }

        Ok(())
    }

    /// Returns the deduplicated list of shared credential keys requested by this plugin.
    pub fn requested_shared_credentials(&self) -> Vec<String> {
        let mut keys = self.permissions.shared_credentials.clone();
        for spec in &self.shared_credentials {
            if !keys.contains(&spec.id) {
                keys.push(spec.id.clone());
            }
        }
        keys
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
        assert_eq!(manifest.permissions.scopes.len(), 2);
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
            long_description: None,
            readme: None,
            icon: None,
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec!["echo".to_string()],
                os: None,
            },
            permissions: PluginPermissions::default(),
            shared_credentials: vec![],
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
            long_description: None,
            readme: None,
            icon: None,
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec!["echo".to_string()],
                os: None,
            },
            permissions: PluginPermissions::default(),
            shared_credentials: vec![],
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
            long_description: None,
            readme: None,
            icon: None,
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec![],
                os: None,
            },
            permissions: PluginPermissions::default(),
            shared_credentials: vec![],
            tab: None,
            settings: None,
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_icon_validation() {
        let mut manifest = PluginManifest {
            id: "icon-test".to_string(),
            name: "Icon Test".to_string(),
            version: "1.0.0".to_string(),
            api_version: 1,
            description: None,
            long_description: None,
            readme: None,
            icon: Some("icon.svg".to_string()),
            author: None,
            homepage: None,
            repository: None,
            entry: PluginEntry {
                command: vec!["echo".to_string()],
                os: None,
            },
            permissions: PluginPermissions::default(),
            shared_credentials: vec![],
            tab: None,
            settings: None,
        };
        assert!(manifest.validate().is_ok());

        // Relative paths and remote URLs are ok
        manifest.icon = Some("assets/logo.png".to_string());
        assert!(manifest.validate().is_ok());
        manifest.icon = Some("data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=".to_string());
        assert!(manifest.validate().is_ok());
        manifest.icon = Some("https://example.com/icon.png".to_string());
        assert!(manifest.validate().is_ok());

        // Traversal is rejected
        manifest.icon = Some("../evil.png".to_string());
        assert!(manifest.validate().is_err());
        manifest.icon = Some("assets/../../etc/passwd".to_string());
        assert!(manifest.validate().is_err());

        // Absolute paths are rejected
        manifest.icon = Some("/root/icon.png".to_string());
        assert!(manifest.validate().is_err());
        manifest.icon = Some("\\windows\\icon.png".to_string());
        assert!(manifest.validate().is_err());
        manifest.icon = Some("C:\\icon.png".to_string());
        assert!(manifest.validate().is_err());

        // Tab icon follows same rules
        manifest.icon = None;
        manifest.tab = Some(PluginTab {
            title: "Test".to_string(),
            icon: Some("../evil.svg".to_string()),
        });
        assert!(manifest.validate().is_err());

        manifest.tab = Some(PluginTab {
            title: "Test".to_string(),
            icon: Some("puzzle".to_string()),
        });
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_shared_credentials_manifest() {
        let json = r#"{
            "id": "cred-plugin",
            "name": "Cred Plugin",
            "version": "1.0.0",
            "apiVersion": 1,
            "entry": {
                "command": ["python3", "main.py"]
            },
            "permissions": {
                "scopes": ["network"],
                "sharedCredentials": ["tmdb_api_key"]
            },
            "sharedCredentials": [
                "opensubtitles_api_key",
                { "id": "imdb_id", "label": "IMDb Key" }
            ]
        }"#;

        let manifest: PluginManifest = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(manifest.permissions.scopes, vec!["network"]);
        assert_eq!(
            manifest.permissions.shared_credentials,
            vec!["tmdb_api_key"]
        );
        assert_eq!(manifest.shared_credentials.len(), 2);
        assert_eq!(manifest.shared_credentials[0].id, "opensubtitles_api_key");
        assert_eq!(manifest.shared_credentials[1].id, "imdb_id");
        assert_eq!(
            manifest.shared_credentials[1].label,
            Some("IMDb Key".to_string())
        );

        let requested = manifest.requested_shared_credentials();
        assert_eq!(requested.len(), 3);
        assert!(requested.contains(&"tmdb_api_key".to_string()));
        assert!(requested.contains(&"opensubtitles_api_key".to_string()));
        assert!(requested.contains(&"imdb_id".to_string()));
        assert!(manifest.validate().is_ok());

        // Test invalid shared credential ID (traversal or special chars)
        let mut invalid_manifest = manifest.clone();
        invalid_manifest.permissions.shared_credentials = vec!["../bad_cred".to_string()];
        assert!(invalid_manifest.validate().is_err());
    }
}

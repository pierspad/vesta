//! Persistent on-disk configuration store (`vesta_config.json`), replacing
//! Webview `localStorage`.
//!
//! `localStorage` lives inside the Webview cache: clearing the cache or
//! upgrading system packages can silently wipe it, losing API keys and custom
//! settings (note types, templates, etc.). Here values are stored in a
//! Rust-managed JSON file outside the Webview cache with atomic writes
//! (write-to-temp + rename) to prevent corruption on crash.
//!
//! The data model is intentionally `HashMap<String, String>` to mirror
//! `localStorage` (string keys and values, with objects JSON-serialized by the
//! caller), allowing the TypeScript layer to act as a 1:1 drop-in replacement.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::State;

/// In-memory configuration store state kept in sync with the on-disk file.
pub struct ConfigState(pub Arc<Mutex<HashMap<String, String>>>);

impl Default for ConfigState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }
}

/// Configuration snapshot returned when loading `vesta_config.json`, including
/// the backup path if a corrupted file had to be quarantined and reset.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConfigLoadResult {
    pub entries: HashMap<String, String>,
    pub recovered_backup_path: Option<String>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("vesta")
}

fn config_file() -> PathBuf {
    config_dir().join("vesta_config.json")
}

fn read_from_disk() -> Result<ConfigLoadResult, String> {
    read_config_file(&config_file())
}

fn next_corrupt_backup_path(path: &std::path::Path, file_name: &str, ts: u64) -> PathBuf {
    let initial = path.with_file_name(format!("{file_name}.corrupt.{ts}"));
    if !initial.exists() {
        return initial;
    }
    for suffix in 1.. {
        let candidate = path.with_file_name(format!("{file_name}.corrupt.{ts}.{suffix}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    initial
}

fn read_config_file(path: &std::path::Path) -> Result<ConfigLoadResult, String> {
    match fs::read_to_string(path) {
        Ok(raw) => match serde_json::from_str(&raw) {
            Ok(entries) => Ok(ConfigLoadResult {
                entries,
                recovered_backup_path: None,
            }),
            Err(e) => {
                eprintln!(
                    "Warning: Corrupted configuration file at {}: {e}. Backing up and resetting.",
                    path.display()
                );
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("vesta_config.json");
                let backup_path = next_corrupt_backup_path(path, file_name, ts);
                fs::rename(path, &backup_path).map_err(|backup_err| {
                    format!(
                        "Corrupted configuration at {} could not be backed up to {}: {backup_err}",
                        path.display(),
                        backup_path.display()
                    )
                })?;
                Ok(ConfigLoadResult {
                    entries: HashMap::new(),
                    recovered_backup_path: Some(backup_path.to_string_lossy().into_owned()),
                })
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLoadResult {
            entries: HashMap::new(),
            recovered_backup_path: None,
        }),
        Err(e) => Err(format!("Failed to read configuration: {e}")),
    }
}

/// Atomic write via temporary file + rename so a crash mid-write never leaves
/// a truncated or corrupted `vesta_config.json`.
fn write_to_disk(map: &HashMap<String, String>) -> Result<(), String> {
    write_config_file(&config_file(), map)
}

fn write_config_file(path: &std::path::Path, map: &HashMap<String, String>) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or("Configuration path has no parent directory")?;
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;

    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;

    let mut temporary = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temporary
        .write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write temporary configuration: {e}"))?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary
        .persist(path)
        .map_err(|e| format!("Failed to persist configuration: {}", e.error))?;
    Ok(())
}

/// Loads (or reloads) the entire configuration store from disk and returns it
/// to the frontend. Called once at startup before mounting the app to hydrate
/// the in-memory TypeScript cache.
#[tauri::command]
pub fn config_load_all(state: State<'_, ConfigState>) -> Result<ConfigLoadResult, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let loaded = read_from_disk()?;
    *guard = loaded.entries.clone();
    Ok(loaded)
}

#[tauri::command]
pub async fn config_set(
    state: State<'_, ConfigState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let store = Arc::clone(&state.0);
    // Atomic writes include fsync; never run them on the webview event thread.
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = store.lock().map_err(|e| e.to_string())?;
        let old_value = guard.insert(key.clone(), value);
        if let Err(e) = write_to_disk(&guard) {
            if let Some(old) = old_value {
                guard.insert(key, old);
            } else {
                guard.remove(&key);
            }
            return Err(e);
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn config_remove(state: State<'_, ConfigState>, key: String) -> Result<(), String> {
    let store = Arc::clone(&state.0);
    // Atomic writes include fsync; never run them on the webview event thread.
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = store.lock().map_err(|e| e.to_string())?;
        let old_value = guard.remove(&key);
        if let Err(e) = write_to_disk(&guard) {
            if let Some(old) = old_value {
                guard.insert(key, old);
            }
            return Err(e);
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn config_clear(state: State<'_, ConfigState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    write_to_disk(&HashMap::new())?;
    guard.clear();
    Ok(())
}

/// Replaces the complete configuration in one atomic write. Used by the
/// settings import flow so a failed import cannot leave a half-applied mix of
/// old and new preferences.
#[tauri::command]
pub fn config_replace_all(
    state: State<'_, ConfigState>,
    values: HashMap<String, String>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    write_to_disk(&values)?;
    *guard = values;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_snapshot_survives_reopen_and_recovers_from_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/settings.json");
        let values = HashMap::from([
            ("setup-complete".into(), "true".into()),
            ("language".into(), "it".into()),
        ]);
        write_config_file(&path, &values).unwrap();
        assert_eq!(
            read_config_file(&path).unwrap(),
            ConfigLoadResult {
                entries: values,
                recovered_backup_path: None,
            }
        );
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        }
        fs::write(&path, "broken json").unwrap();
        let recovered = read_config_file(&path).unwrap();
        assert!(recovered.entries.is_empty());
        let backup_path = recovered
            .recovered_backup_path
            .expect("corrupted config must report backup path");
        assert!(PathBuf::from(&backup_path).exists());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "broken json");
        let parent_entries: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            parent_entries
                .iter()
                .any(|name| name.starts_with("settings.json.corrupt."))
        );
    }

    #[cfg(unix)]
    #[test]
    fn corrupted_config_aborts_reset_when_backup_cannot_be_created() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("locked");
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("settings.json");
        fs::write(&path, "broken json with api key").unwrap();

        fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
        let probe = parent.join("write_probe");
        let enforced_readonly = fs::write(&probe, b"").is_err();
        let _ = fs::remove_file(&probe);

        let result = read_config_file(&path);
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();

        if enforced_readonly {
            let error = result.expect_err("reset must fail when backup cannot be written");
            assert!(error.contains("could not be backed up"), "{error}");
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "broken json with api key"
            );
        }
    }
}

#[tauri::command]
pub fn config_reset_installation(
    app: tauri::AppHandle,
    state: State<'_, ConfigState>,
) -> Result<(), String> {
    use tauri::Manager;
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    let temp_subs = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("temp_subs");
    for path in [cache, temp_subs] {
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    config_clear(state)
}

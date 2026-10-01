//! Config store persistito su disco (`vesta_config.json`), sostituto del
//! `localStorage` del Webview.
//!
//! `localStorage` vive nella cache del Webview: una pulizia della cache o un
//! aggiornamento di sistema può cancellarlo silenziosamente, perdendo chiavi
//! API e impostazioni personalizzate (note types, template, ecc.). Qui i
//! valori vivono in un file JSON gestito da Rust, fuori dalla cache del
//! Webview, con scrittura atomica (write-to-temp + rename) per evitare file
//! corrotti in caso di crash a metà scrittura.
//!
//! Il modello è intenzionalmente `HashMap<String, String>`: rispecchia
//! `localStorage` (chiave/valore, solo stringhe — gli oggetti restano
//! serializzati in JSON dal chiamante) così il lato TypeScript può restare
//! un drop-in replacement 1:1 delle chiamate `localStorage.*`.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

/// Stato in-memory del config store, tenuto in sync col file su disco.
pub struct ConfigState(pub Mutex<HashMap<String, String>>);

impl Default for ConfigState {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("vesta")
}

fn config_file() -> PathBuf {
    config_dir().join("vesta_config.json")
}

fn read_from_disk() -> Result<HashMap<String, String>, String> {
    read_config_file(&config_file())
}

fn read_config_file(path: &std::path::Path) -> Result<HashMap<String, String>, String> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|e| format!("Configurazione non valida: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(format!("Impossibile leggere la configurazione: {e}")),
    }
}

/// Scrittura atomica: file temporaneo + rename, così un crash a metà
/// scrittura non lascia mai un `vesta_config.json` troncato/corrotto.
fn write_to_disk(map: &HashMap<String, String>) -> Result<(), String> {
    write_config_file(&config_file(), map)
}

fn write_config_file(path: &std::path::Path, map: &HashMap<String, String>) -> Result<(), String> {
    let dir = path.parent().ok_or("Configurazione senza cartella")?;
    fs::create_dir_all(dir).map_err(|e| format!("impossibile creare {}: {e}", dir.display()))?;

    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;

    let mut temporary = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temporary
        .write_all(json.as_bytes())
        .map_err(|e| format!("scrittura fallita: {e}"))?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary
        .persist(path)
        .map_err(|e| format!("salvataggio fallito: {}", e.error))?;
    Ok(())
}

/// Carica (o ricarica) l'intero config store dal disco e lo restituisce al
/// frontend. Chiamato una sola volta all'avvio, prima del mount dell'app,
/// per idratare la cache in-memory lato TypeScript.
#[tauri::command]
pub fn config_load_all(state: State<ConfigState>) -> Result<HashMap<String, String>, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = read_from_disk()?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn config_set(state: State<ConfigState>, key: String, value: String) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let mut next = guard.clone();
    next.insert(key, value);
    write_to_disk(&next)?;
    *guard = next;
    Ok(())
}

#[tauri::command]
pub fn config_remove(state: State<ConfigState>, key: String) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let mut next = guard.clone();
    next.remove(&key);
    write_to_disk(&next)?;
    *guard = next;
    Ok(())
}

#[tauri::command]
pub fn config_clear(state: State<ConfigState>) -> Result<(), String> {
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
    state: State<ConfigState>,
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
    fn settings_snapshot_survives_reopen_and_reports_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/settings.json");
        let values = HashMap::from([
            ("setup-complete".into(), "true".into()),
            ("language".into(), "it".into()),
        ]);
        write_config_file(&path, &values).unwrap();
        assert_eq!(read_config_file(&path).unwrap(), values);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        }
        fs::write(&path, "broken json").unwrap();
        assert!(read_config_file(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "broken json");
    }
}

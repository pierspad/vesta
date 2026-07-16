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

fn read_from_disk() -> HashMap<String, String> {
    fs::read_to_string(config_file())
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Scrittura atomica: file temporaneo + rename, così un crash a metà
/// scrittura non lascia mai un `vesta_config.json` troncato/corrotto.
fn write_to_disk(map: &HashMap<String, String>) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("impossibile creare {}: {e}", dir.display()))?;

    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;

    let tmp_path = dir.join("vesta_config.json.tmp");
    fs::write(&tmp_path, json).map_err(|e| format!("scrittura fallita: {e}"))?;
    fs::rename(&tmp_path, config_file()).map_err(|e| format!("rename fallito: {e}"))?;
    Ok(())
}

/// Carica (o ricarica) l'intero config store dal disco e lo restituisce al
/// frontend. Chiamato una sola volta all'avvio, prima del mount dell'app,
/// per idratare la cache in-memory lato TypeScript.
#[tauri::command]
pub fn config_load_all(state: State<ConfigState>) -> HashMap<String, String> {
    let mut guard = state.0.lock().unwrap();
    *guard = read_from_disk();
    guard.clone()
}

#[tauri::command]
pub fn config_set(state: State<ConfigState>, key: String, value: String) -> Result<(), String> {
    let mut guard = state.0.lock().unwrap();
    guard.insert(key, value);
    write_to_disk(&guard)
}

#[tauri::command]
pub fn config_remove(state: State<ConfigState>, key: String) -> Result<(), String> {
    let mut guard = state.0.lock().unwrap();
    guard.remove(&key);
    write_to_disk(&guard)
}

#[tauri::command]
pub fn config_clear(state: State<ConfigState>) -> Result<(), String> {
    let mut guard = state.0.lock().unwrap();
    guard.clear();
    write_to_disk(&guard)
}

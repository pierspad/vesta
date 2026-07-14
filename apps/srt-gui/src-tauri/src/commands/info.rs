//! Comandi Tauri per informazioni sull'applicazione e utilità sui file.

use serde::{Deserialize, Serialize};

/// Legge un file di sottotitoli rilevandone automaticamente l'encoding
/// (BOM, UTF-8/16, code page legacy come GBK o Windows-1252) e lo
/// restituisce come testo UTF-8 pulito.
///
/// Il frontend deve usare questo comando al posto di `readTextFile` del
/// plugin fs, che fallisce (o produce mojibake) sui file non UTF-8.
#[tauri::command]
pub fn read_subtitle_file(path: String) -> Result<String, String> {
    srt_parser::encoding::read_text_auto(&path).map_err(|e| e.to_string())
}

/// Informazioni sull'applicazione
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub license: String,
}

/// Ritorna le informazioni sull'applicazione
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "VESTA".to_string(),
        license: env!("CARGO_PKG_LICENSE").to_string(),
    }
}

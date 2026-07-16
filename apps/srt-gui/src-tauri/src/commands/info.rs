use serde::{Deserialize, Serialize};

#[tauri::command]
pub fn read_subtitle_file(path: String) -> Result<String, String> {
    srt_parser::encoding::read_text_auto(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub name: String,
    pub license: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "vesta".to_string(),
        license: env!("CARGO_PKG_LICENSE").to_string(),
    }
}

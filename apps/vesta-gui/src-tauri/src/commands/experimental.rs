fn anki_url(url: Option<String>) -> String {
    url.filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| vesta_ankiconnect::DEFAULT_URL.to_string())
}

#[tauri::command]
pub async fn ankiconnect_ping(url: Option<String>) -> Result<u32, String> {
    vesta_ankiconnect::ping(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_deck_names(url: Option<String>) -> Result<Vec<String>, String> {
    vesta_ankiconnect::deck_names(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_import_package(path: String, url: Option<String>) -> Result<bool, String> {
    vesta_ankiconnect::import_package(&anki_url(url), &path).await?;
    Ok(true)
}

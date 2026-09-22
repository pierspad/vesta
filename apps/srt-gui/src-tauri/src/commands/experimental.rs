fn anki_url(url: Option<String>) -> String {
    url.filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| srt_ankiconnect::DEFAULT_URL.to_string())
}

#[tauri::command]
pub async fn ankiconnect_ping(url: Option<String>) -> Result<u32, String> {
    srt_ankiconnect::ping(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_deck_names(url: Option<String>) -> Result<Vec<String>, String> {
    srt_ankiconnect::deck_names(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_import_package(path: String, url: Option<String>) -> Result<bool, String> {
    srt_ankiconnect::import_package(&anki_url(url), &path).await?;
    Ok(true)
}

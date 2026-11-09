use crate::plugins::registry::InstalledPlugin;
use crate::plugins::{PluginEventPayload, PluginInspectResult, PluginManager};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};

pub type AppPluginState = Arc<PluginManager>;

#[tauri::command]
pub async fn plugin_inspect(
    state: State<'_, AppPluginState>,
    url: String,
    git_ref: Option<String>,
) -> Result<PluginInspectResult, String> {
    state.inspect(&url, git_ref.as_deref()).await
}

#[tauri::command]
pub async fn plugin_install(
    state: State<'_, AppPluginState>,
    url: String,
    git_ref: Option<String>,
) -> Result<InstalledPlugin, String> {
    state.install(&url, git_ref.as_deref()).await
}

#[tauri::command]
pub async fn plugin_list(state: State<'_, AppPluginState>) -> Result<Vec<InstalledPlugin>, String> {
    Ok(state.list().await)
}

#[tauri::command]
pub async fn plugin_enable(state: State<'_, AppPluginState>, id: String) -> Result<(), String> {
    state.enable(&id).await
}

#[tauri::command]
pub async fn plugin_disable(state: State<'_, AppPluginState>, id: String) -> Result<(), String> {
    state.disable(&id).await
}

#[tauri::command]
pub async fn plugin_uninstall(state: State<'_, AppPluginState>, id: String) -> Result<(), String> {
    state.uninstall(&id).await
}

#[tauri::command]
pub async fn plugin_update(
    state: State<'_, AppPluginState>,
    id: String,
    release_tag: Option<String>,
) -> Result<InstalledPlugin, String> {
    state.update(&id, release_tag.as_deref()).await
}

#[tauri::command]
pub async fn plugin_get_settings(
    state: State<'_, AppPluginState>,
    id: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    state.get_settings(&id).await
}

#[tauri::command]
pub async fn plugin_save_settings(
    state: State<'_, AppPluginState>,
    id: String,
    settings: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    state.save_settings(&id, settings).await
}

#[tauri::command]
pub async fn plugin_render(
    app: AppHandle,
    state: State<'_, AppPluginState>,
    plugin_id: String,
    screen: Option<String>,
    params: Option<serde_json::Value>,
    locale: Option<String>,
    expert_mode: Option<bool>,
) -> Result<serde_json::Value, String> {
    let screen_name = screen.unwrap_or_else(|| "main".to_string());
    let params_val = params.unwrap_or(serde_json::Value::Null);
    let loc = locale.unwrap_or_else(|| "en".to_string());
    let is_expert = expert_mode.unwrap_or(false);
    state
        .render(
            Some(app),
            &plugin_id,
            &screen_name,
            params_val,
            &loc,
            is_expert,
        )
        .await
}

#[tauri::command]
pub async fn plugin_dispatch_event(
    app: AppHandle,
    state: State<'_, AppPluginState>,
    plugin_id: String,
    event: PluginEventPayload,
    locale: Option<String>,
    expert_mode: Option<bool>,
) -> Result<serde_json::Value, String> {
    let loc = locale.unwrap_or_else(|| "en".to_string());
    let is_expert = expert_mode.or(event.expert_mode).unwrap_or(false);
    state
        .dispatch_event(Some(app), &plugin_id, &event, &loc, is_expert)
        .await
}

#[tauri::command]
pub async fn plugin_list_shared_credentials(
    state: State<'_, AppPluginState>,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    Ok(state.list_shared_credentials().await)
}

#[tauri::command]
pub async fn plugin_set_shared_credential(
    state: State<'_, AppPluginState>,
    key: String,
    value: String,
) -> Result<(), String> {
    state.set_shared_credential(key, Some(value)).await
}

#[tauri::command]
pub async fn plugin_delete_shared_credential(
    state: State<'_, AppPluginState>,
    key: String,
) -> Result<(), String> {
    state.set_shared_credential(key, None).await
}

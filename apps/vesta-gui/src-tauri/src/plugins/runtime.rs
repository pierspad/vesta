use crate::plugins::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, Notify, mpsc, oneshot};

pub(crate) const MAX_PLUGIN_RPC_LINE_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_PLUGIN_STDERR_LINE_BYTES: usize = 64 * 1024;
const DEFAULT_INIT_TIMEOUT: Duration = Duration::from_secs(10);

const BASE_ENV_ALLOWLIST: &[&str] = &[
    "PATH", "TMPDIR", "TEMP", "TMP", "LANG", "LC_ALL", "LC_CTYPE",
];

#[cfg(not(target_os = "windows"))]
const PLATFORM_ENV_ALLOWLIST: &[&str] = &["HOME"];

#[cfg(target_os = "windows")]
const PLATFORM_ENV_ALLOWLIST: &[&str] = &[
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
];

const NETWORK_ENV_ALLOWLIST: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "REQUESTS_CA_BUNDLE",
    "CURL_CA_BUNDLE",
];

fn normalize_plugin_scopes(plugin_id: &str, raw_scopes: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for raw in raw_scopes {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(format!(
                "Plugin '{plugin_id}' declares an empty permission scope"
            ));
        }
        let canonical = match trimmed.to_ascii_lowercase().as_str() {
            "network" => "network",
            "filesystem:downloads" => "filesystem:downloads",
            _ => {
                return Err(format!(
                    "Plugin '{plugin_id}' declares unsupported permission scope '{trimmed}'"
                ));
            }
        };
        if !normalized.iter().any(|existing| existing == canonical) {
            normalized.push(canonical.to_string());
        }
    }
    Ok(normalized)
}

fn resolve_downloads_dir(data_dir: &Path) -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| data_dir.join("downloads"))
}

fn build_plugin_env<F>(
    plugin_id: &str,
    data_dir: &Path,
    scopes: &[String],
    downloads_dir: Option<&Path>,
    mut get_parent_env: F,
) -> Vec<(std::ffi::OsString, std::ffi::OsString)>
where
    F: FnMut(&str) -> Option<std::ffi::OsString>,
{
    let has_network_scope = scopes.iter().any(|s| s == "network");
    let mut env_pairs = Vec::new();
    for key in BASE_ENV_ALLOWLIST
        .iter()
        .chain(PLATFORM_ENV_ALLOWLIST.iter())
    {
        if let Some(val) = get_parent_env(key) {
            env_pairs.push((std::ffi::OsString::from(key), val));
        }
    }
    if has_network_scope {
        for key in NETWORK_ENV_ALLOWLIST {
            if let Some(val) = get_parent_env(key) {
                env_pairs.push((std::ffi::OsString::from(key), val));
            }
        }
    }
    env_pairs.push((
        std::ffi::OsString::from("PYTHONUTF8"),
        std::ffi::OsString::from("1"),
    ));
    env_pairs.push((
        std::ffi::OsString::from("PYTHONDONTWRITEBYTECODE"),
        std::ffi::OsString::from("1"),
    ));
    env_pairs.push((
        std::ffi::OsString::from("VESTA_PLUGIN_ID"),
        std::ffi::OsString::from(plugin_id),
    ));
    env_pairs.push((
        std::ffi::OsString::from("VESTA_API_VERSION"),
        std::ffi::OsString::from("1"),
    ));
    env_pairs.push((
        std::ffi::OsString::from("VESTA_DATA_DIR"),
        data_dir.as_os_str().to_os_string(),
    ));
    env_pairs.push((
        std::ffi::OsString::from("VESTA_PLUGIN_SCOPES"),
        std::ffi::OsString::from(scopes.join(",")),
    ));
    if let Some(dl_dir) = downloads_dir {
        env_pairs.push((
            std::ffi::OsString::from("VESTA_DOWNLOADS_DIR"),
            dl_dir.as_os_str().to_os_string(),
        ));
    }
    env_pairs
}

pub(crate) async fn read_bounded_rpc_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<String>, String> {
    let mut buf = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(|e| e.to_string())?;
        if available.is_empty() {
            return if buf.is_empty() {
                Ok(None)
            } else {
                String::from_utf8(buf).map(Some).map_err(|e| e.to_string())
            };
        }
        let newline_pos = available.iter().position(|&b| b == b'\n');
        let take_len = newline_pos.map_or(available.len(), |pos| pos + 1);
        if buf.len().saturating_add(take_len) > max_bytes {
            return Err(format!(
                "Plugin RPC response line exceeded {max_bytes} byte limit"
            ));
        }
        buf.extend_from_slice(&available[..take_len]);
        reader.consume(take_len);
        if newline_pos.is_some() {
            while matches!(buf.last(), Some(b'\n' | b'\r')) {
                buf.pop();
            }
            return String::from_utf8(buf).map(Some).map_err(|e| e.to_string());
        }
    }
}

pub(crate) async fn read_bounded_stderr_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<(String, bool)>, std::io::Error> {
    let mut buf = Vec::new();
    let mut truncated = false;
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if buf.is_empty() && !truncated {
                return Ok(None);
            }
            while matches!(buf.last(), Some(b'\n' | b'\r')) {
                buf.pop();
            }
            return Ok(Some((
                String::from_utf8_lossy(&buf).into_owned(),
                truncated,
            )));
        }
        let newline_pos = available.iter().position(|&b| b == b'\n');
        let take_len = newline_pos.map_or(available.len(), |pos| pos + 1);
        if !truncated {
            let remaining = max_bytes.saturating_sub(buf.len());
            let copy_len = take_len.min(remaining);
            buf.extend_from_slice(&available[..copy_len]);
            if copy_len < take_len {
                truncated = true;
            }
        }
        reader.consume(take_len);
        if newline_pos.is_some() {
            while matches!(buf.last(), Some(b'\n' | b'\r')) {
                buf.pop();
            }
            return Ok(Some((
                String::from_utf8_lossy(&buf).into_owned(),
                truncated,
            )));
        }
    }
}

async fn kill_and_reap_child(
    child_handle: &std::sync::Mutex<Option<tokio::process::Child>>,
    exited: &AtomicBool,
) {
    exited.store(true, Ordering::SeqCst);
    let child = {
        let mut guard = child_handle.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };
    if let Some(mut child) = child {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

pub type PendingRequests =
    Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>>;

#[allow(dead_code)]
pub struct ActivePluginProcess {
    pub plugin_id: String,
    pub plugin_dir: PathBuf,
    stdin_tx: mpsc::Sender<String>,
    pending_requests: PendingRequests,
    pub latest_values: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    next_id: AtomicU64,
    initialized: AtomicBool,
    exited: Arc<AtomicBool>,
    child_handle: Arc<std::sync::Mutex<Option<tokio::process::Child>>>,
}

impl ActivePluginProcess {
    pub fn is_alive(&self) -> bool {
        if self.exited.load(Ordering::SeqCst) || self.stdin_tx.is_closed() {
            return false;
        }
        let mut guard = self.child_handle.lock().unwrap_or_else(|e| e.into_inner());
        let Some(child) = guard.as_mut() else {
            self.exited.store(true, Ordering::SeqCst);
            return false;
        };
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) | Err(_) => {
                self.exited.store(true, Ordering::SeqCst);
                false
            }
        }
    }
}

#[derive(Default)]
struct PluginSlot {
    init_mutex: Mutex<()>,
    generation: AtomicU64,
    stop_notify: Notify,
}

pub struct PluginRuntimeManager {
    processes: Arc<Mutex<HashMap<String, Arc<ActivePluginProcess>>>>,
    slots: Arc<Mutex<HashMap<String, Arc<PluginSlot>>>>,
}

impl Default for PluginRuntimeManager {
    fn default() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            slots: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl PluginRuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    async fn slot_for(&self, plugin_id: &str) -> Arc<PluginSlot> {
        let mut slots = self.slots.lock().await;
        slots
            .entry(plugin_id.to_string())
            .or_insert_with(|| Arc::new(PluginSlot::default()))
            .clone()
    }

    /// Stops and removes any active process for the given plugin.
    pub async fn stop_plugin(&self, plugin_id: &str) {
        let slot = {
            let slots = self.slots.lock().await;
            slots.get(plugin_id).cloned()
        };
        if let Some(slot) = slot {
            slot.generation.fetch_add(1, Ordering::SeqCst);
            slot.stop_notify.notify_waiters();
        }

        let process = {
            let mut procs = self.processes.lock().await;
            procs.remove(plugin_id)
        };

        if let Some(proc) = process {
            // Attempt graceful shutdown
            let _ = Self::send_raw_request(
                &proc,
                "shutdown",
                serde_json::json!({}),
                Duration::from_millis(500),
            )
            .await;

            kill_and_reap_child(&proc.child_handle, &proc.exited).await;
            let mut pending = proc.pending_requests.lock().await;
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err("Plugin stopped".to_string()));
            }
        }
    }

    /// Ensures the plugin child process is running, spawning it if necessary.
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_running(
        &self,
        manifest: &PluginManifest,
        plugin_dir: &Path,
        app_handle: Option<AppHandle>,
        locale: &str,
        settings: &HashMap<String, serde_json::Value>,
        expert_mode: bool,
    ) -> Result<Arc<ActivePluginProcess>, String> {
        self.ensure_running_with_init_timeout(
            manifest,
            plugin_dir,
            app_handle,
            locale,
            settings,
            expert_mode,
            DEFAULT_INIT_TIMEOUT,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ensure_running_with_init_timeout(
        &self,
        manifest: &PluginManifest,
        plugin_dir: &Path,
        app_handle: Option<AppHandle>,
        locale: &str,
        settings: &HashMap<String, serde_json::Value>,
        expert_mode: bool,
        init_timeout: Duration,
    ) -> Result<Arc<ActivePluginProcess>, String> {
        {
            let mut procs = self.processes.lock().await;
            if let Some(existing) = procs.get(&manifest.id) {
                if existing.is_alive() {
                    return Ok(existing.clone());
                }
                procs.remove(&manifest.id);
            }
        }

        let slot = self.slot_for(&manifest.id).await;
        let _init_guard = slot.init_mutex.lock().await;

        {
            let mut procs = self.processes.lock().await;
            if let Some(existing) = procs.get(&manifest.id) {
                if existing.is_alive() {
                    return Ok(existing.clone());
                }
                procs.remove(&manifest.id);
            }
        }

        let start_gen = slot.generation.load(Ordering::SeqCst);
        let scopes = normalize_plugin_scopes(&manifest.id, &manifest.permissions.scopes)?;
        let has_downloads_scope = scopes.iter().any(|s| s == "filesystem:downloads");

        // Spawn child process
        let command = manifest.resolve_command();
        if command.is_empty() {
            return Err("Plugin manifest specifies empty entry command".to_string());
        }

        let executable = &command[0];
        let args = &command[1..];

        let data_dir = plugin_dir.join("data");
        let _ = tokio::fs::create_dir_all(&data_dir).await;
        let downloads_dir = has_downloads_scope.then(|| resolve_downloads_dir(&data_dir));

        let mut cmd = tokio::process::Command::new(executable);
        cmd.args(args)
            .kill_on_drop(true)
            .current_dir(plugin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();

        for (key, val) in build_plugin_env(
            &manifest.id,
            &data_dir,
            &scopes,
            downloads_dir.as_deref(),
            |k| std::env::var_os(k),
        ) {
            cmd.env(key, val);
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn plugin '{}' ({}): {e}",
                manifest.id, executable
            )
        })?;

        let Some(stdin) = child.stdin.take() else {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("Failed to capture stdin for plugin process".to_string());
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("Failed to capture stdout for plugin process".to_string());
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err("Failed to capture stderr for plugin process".to_string());
        };

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);
        let pending = Arc::new(Mutex::new(HashMap::<
            u64,
            oneshot::Sender<Result<serde_json::Value, String>>,
        >::new()));
        let latest_values = Arc::new(Mutex::new(HashMap::<String, serde_json::Value>::new()));
        let exited = Arc::new(AtomicBool::new(false));
        let child_handle = Arc::new(std::sync::Mutex::new(Some(child)));

        // Stdin writer task
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = stdin_rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Stdout reader task (JSON-RPC)
        let pending_clone = pending.clone();
        let plugin_id = manifest.id.clone();
        let app_handle_clone = app_handle.clone();
        let stdin_tx_clone = stdin_tx.clone();
        let latest_values_clone = latest_values.clone();
        let exited_clone = exited.clone();
        let child_handle_clone = child_handle.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut fatal_error: Option<String> = None;
            loop {
                match read_bounded_rpc_line(&mut reader, MAX_PLUGIN_RPC_LINE_BYTES).await {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if let Ok(msg) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                            // Check if message is a request from plugin to host (has id AND method)
                            if let Some(method) = &msg.method
                                && let Some(id) = msg.id
                                && method == "ui.getValues"
                            {
                                let screen = msg
                                    .params
                                    .as_ref()
                                    .and_then(|p| p.get("screen"))
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("main");
                                let vals = latest_values_clone.lock().await;
                                let screen_vals =
                                    vals.get(screen).cloned().unwrap_or(serde_json::json!({}));
                                let resp = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "values": screen_vals }
                                });
                                if let Ok(resp_line) = serde_json::to_string(&resp) {
                                    let _ = stdin_tx_clone.send(resp_line + "\n").await;
                                }
                                continue;
                            }

                            // Check if response has an ID (matching a pending request)
                            if let Some(id) = msg.id {
                                let sender = {
                                    let mut map = pending_clone.lock().await;
                                    map.remove(&id)
                                };
                                if let Some(tx) = sender {
                                    if let Some(err) = msg.error {
                                        let _ = tx.send(Err(format!(
                                            "[RPC Error {}] {}",
                                            err.code, err.message
                                        )));
                                    } else {
                                        let _ = tx.send(Ok(msg
                                            .result
                                            .unwrap_or(serde_json::Value::Null)));
                                    }
                                }
                            } else if let Some(method) = msg.method {
                                // It's a notification from plugin -> host
                                let params = msg.params.unwrap_or(serde_json::Value::Null);
                                Self::handle_notification(
                                    &plugin_id,
                                    &method,
                                    params,
                                    app_handle_clone.as_ref(),
                                );
                            }
                        } else {
                            eprintln!("[plugin:{plugin_id}] non-rpc stdout: {trimmed}");
                        }
                    }
                    Ok(None) => {
                        exited_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    Err(err) => {
                        eprintln!("[plugin:{plugin_id}] stdout reader error: {err}");
                        fatal_error = Some(err);
                        exited_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }

            let drain_msg = fatal_error
                .clone()
                .unwrap_or_else(|| "Plugin process closed stdout unexpectedly".to_string());
            {
                let mut map = pending_clone.lock().await;
                for (_, tx) in map.drain() {
                    let _ = tx.send(Err(drain_msg.clone()));
                }
            }
            if fatal_error.is_some() {
                kill_and_reap_child(&child_handle_clone, &exited_clone).await;
            }
        });

        // Bounded stderr logger task
        let plugin_id_err = manifest.id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            while let Ok(Some((line, was_truncated))) =
                read_bounded_stderr_line(&mut reader, MAX_PLUGIN_STDERR_LINE_BYTES).await
            {
                if was_truncated {
                    eprintln!("[plugin:{plugin_id_err}:stderr] {line}... [truncated]");
                } else {
                    eprintln!("[plugin:{plugin_id_err}:stderr] {line}");
                }
            }
        });

        let active = Arc::new(ActivePluginProcess {
            plugin_id: manifest.id.clone(),
            plugin_dir: plugin_dir.to_path_buf(),
            stdin_tx,
            pending_requests: pending,
            latest_values,
            next_id: AtomicU64::new(1),
            initialized: AtomicBool::new(false),
            exited,
            child_handle,
        });

        // Initialize plugin outside `self.processes` lock
        let mut permissions_obj = serde_json::Map::new();
        permissions_obj.insert(
            "scopes".to_string(),
            serde_json::Value::Array(scopes.into_iter().map(serde_json::Value::String).collect()),
        );
        if let Some(ref dl_dir) = downloads_dir {
            permissions_obj.insert(
                "downloadsDir".to_string(),
                serde_json::Value::String(dl_dir.to_string_lossy().into_owned()),
            );
        }

        let init_params = serde_json::json!({
            "apiVersion": 1,
            "dataDir": data_dir.to_string_lossy(),
            "locale": locale,
            "settings": settings,
            "expertMode": expert_mode,
            "permissions": serde_json::Value::Object(permissions_obj),
        });

        let init_res = tokio::select! {
            res = Self::send_raw_request(&active, "initialize", init_params, init_timeout) => res,
            _ = async {
                loop {
                    let notified = slot.stop_notify.notified();
                    tokio::pin!(notified);
                    notified.as_mut().enable();
                    if slot.generation.load(Ordering::SeqCst) != start_gen {
                        break;
                    }
                    notified.as_mut().await;
                }
            } => Err("Plugin initialization cancelled by stop_plugin".to_string()),
        };

        if let Err(e) = init_res {
            kill_and_reap_child(&active.child_handle, &active.exited).await;
            return Err(format!(
                "Plugin '{}' failed to initialize: {e}",
                manifest.id
            ));
        }

        let mut procs = self.processes.lock().await;
        if slot.generation.load(Ordering::SeqCst) != start_gen {
            drop(procs);
            kill_and_reap_child(&active.child_handle, &active.exited).await;
            return Err(format!(
                "Plugin '{}' was stopped during initialization",
                manifest.id
            ));
        }

        active.initialized.store(true, Ordering::SeqCst);
        procs.insert(manifest.id.clone(), active.clone());
        Ok(active)
    }

    /// Handles notifications received from the plugin child process.
    fn handle_notification(
        plugin_id: &str,
        method: &str,
        params: serde_json::Value,
        app_handle: Option<&AppHandle>,
    ) {
        if let Some(app) = app_handle {
            match method {
                "ui.notify" => {
                    let level = params
                        .get("level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("info");
                    let msg = params.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                    let _ = app.emit(
                        "plugin:notify",
                        serde_json::json!({
                            "pluginId": plugin_id,
                            "level": level,
                            "msg": msg
                        }),
                    );
                }
                "ui.progress" => {
                    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let value = params.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let label = params.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let _ = app.emit(
                        "plugin:progress",
                        serde_json::json!({
                            "pluginId": plugin_id,
                            "id": id,
                            "value": value,
                            "label": label
                        }),
                    );
                }
                "ui.update" => {
                    let screen = params
                        .get("screen")
                        .and_then(|v| v.as_str())
                        .unwrap_or("main");
                    let root = params
                        .get("root")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let patch = params
                        .get("patch")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let _ = app.emit(
                        "plugin:update",
                        serde_json::json!({
                            "pluginId": plugin_id,
                            "screen": screen,
                            "root": root,
                            "patch": patch
                        }),
                    );
                }
                "vesta.openInFlashcards" => {
                    let video_path = params
                        .get("videoPath")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let subtitle_path = params
                        .get("subtitlePath")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let _ = app.emit(
                        "plugin:openInFlashcards",
                        serde_json::json!({
                            "videoPath": video_path,
                            "subtitlePath": subtitle_path
                        }),
                    );
                }
                _ => {
                    eprintln!("[plugin:{plugin_id}] Unhandled notification: {method}");
                }
            }
        }
    }

    /// Sends a JSON-RPC request to the running child process and awaits its result.
    async fn send_raw_request(
        process: &ActivePluginProcess,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        if process.exited.load(Ordering::SeqCst) {
            return Err("Plugin process has already exited".to_string());
        }
        let req_id = process.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            method: method.to_string(),
            params,
        };

        let json_line = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize RPC request: {e}"))?
            + "\n";

        let (tx, rx) = oneshot::channel();
        {
            let mut map = process.pending_requests.lock().await;
            if process.exited.load(Ordering::SeqCst) {
                return Err("Plugin process has already exited".to_string());
            }
            map.insert(req_id, tx);
        }

        if process.stdin_tx.send(json_line).await.is_err() {
            let mut map = process.pending_requests.lock().await;
            map.remove(&req_id);
            return Err("Failed to write to plugin stdin (process closed)".to_string());
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Plugin process dropped request receiver".to_string()),
            Err(_) => {
                let mut map = process.pending_requests.lock().await;
                map.remove(&req_id);
                Err(format!("RPC call '{method}' timed out after {timeout:?}"))
            }
        }
    }

    /// Invokes `render` on the plugin child process.
    pub async fn render(
        &self,
        process: &ActivePluginProcess,
        screen: &str,
        params: serde_json::Value,
        locale: &str,
        expert_mode: bool,
    ) -> Result<serde_json::Value, String> {
        if params.is_object() {
            let mut vals = process.latest_values.lock().await;
            vals.entry(screen.to_string())
                .or_insert_with(|| params.clone());
        }
        Self::send_raw_request(
            process,
            "render",
            serde_json::json!({
                "screen": screen,
                "params": params,
                "locale": locale,
                "expertMode": expert_mode,
            }),
            Duration::from_secs(15),
        )
        .await
    }

    /// Invokes `event` on the plugin child process.
    #[allow(clippy::too_many_arguments)]
    pub async fn dispatch_event(
        &self,
        process: &ActivePluginProcess,
        screen: &str,
        widget_id: &str,
        event_type: &str,
        values: serde_json::Value,
        locale: &str,
        expert_mode: bool,
    ) -> Result<serde_json::Value, String> {
        {
            let mut vals = process.latest_values.lock().await;
            vals.insert(screen.to_string(), values.clone());
        }

        Self::send_raw_request(
            process,
            "event",
            serde_json::json!({
                "screen": screen,
                "widgetId": widget_id,
                "type": event_type,
                "values": values,
                "locale": locale,
                "expertMode": expert_mode,
            }),
            Duration::from_secs(30),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::{PluginEntry, PluginPermissions};

    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct EnvRestore(Vec<(&'static str, Option<std::ffi::OsString>)>);

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, prev) in self.0.drain(..).rev() {
                // SAFETY: tests holding ENV_LOCK serialize all process env mutations.
                unsafe {
                    match prev {
                        Some(val) => std::env::set_var(key, val),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn python_manifest(id: &str, script: &str, scopes: Vec<&str>) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            name: format!("Test {id}"),
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
                command: vec![
                    "python3".to_string(),
                    "-u".to_string(),
                    "-c".to_string(),
                    script.to_string(),
                ],
                os: None,
            },
            permissions: PluginPermissions {
                scopes: scopes.into_iter().map(str::to_string).collect(),
            },
            tab: None,
            settings: None,
        }
    }

    #[tokio::test]
    async fn env_isolation_strips_parent_secrets_and_passes_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = PluginRuntimeManager::new();

        let script = r#"
import json, os, sys
init_params = {}
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        init_params = req.get("params", {})
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}), flush=True)
    elif m == "render":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"env": dict(os.environ), "init": init_params}}), flush=True)
    elif m == "shutdown":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": None}), flush=True)
        break
"#;

        // Pure env-allowlist & proxy-gating check (avoids mutating global HTTP_PROXY during parallel tests).
        let synthetic_parent: HashMap<&str, &str> = HashMap::from([
            ("PATH", "/usr/bin"),
            ("VESTA_TEST_SECRET", "top-secret"),
            ("OPENAI_API_KEY", "sk-test"),
            ("HTTP_PROXY", "http://127.0.0.1:9999"),
            ("HTTPS_PROXY", "http://127.0.0.1:9999"),
            ("REQUESTS_CA_BUNDLE", "/tmp/ca.pem"),
        ]);
        let dl_env_map: HashMap<String, String> = build_plugin_env(
            "env-dl",
            dir.path(),
            &["filesystem:downloads".to_string()],
            Some(&dir.path().join("downloads")),
            |k| synthetic_parent.get(k).map(std::ffi::OsString::from),
        )
        .into_iter()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.to_string_lossy().into_owned(),
            )
        })
        .collect();
        assert_eq!(dl_env_map.get("PATH").map(String::as_str), Some("/usr/bin"));
        assert!(!dl_env_map.contains_key("VESTA_TEST_SECRET"));
        assert!(!dl_env_map.contains_key("OPENAI_API_KEY"));
        assert!(!dl_env_map.contains_key("HTTP_PROXY"));
        assert!(!dl_env_map.contains_key("HTTPS_PROXY"));
        assert!(!dl_env_map.contains_key("REQUESTS_CA_BUNDLE"));
        assert!(dl_env_map.contains_key("VESTA_DOWNLOADS_DIR"));

        let net_env_map: HashMap<String, String> =
            build_plugin_env("env-net", dir.path(), &["network".to_string()], None, |k| {
                synthetic_parent.get(k).map(std::ffi::OsString::from)
            })
            .into_iter()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.to_string_lossy().into_owned(),
                )
            })
            .collect();
        assert_eq!(
            net_env_map.get("HTTP_PROXY").map(String::as_str),
            Some("http://127.0.0.1:9999")
        );
        assert_eq!(
            net_env_map.get("HTTPS_PROXY").map(String::as_str),
            Some("http://127.0.0.1:9999")
        );
        assert_eq!(
            net_env_map.get("REQUESTS_CA_BUNDLE").map(String::as_str),
            Some("/tmp/ca.pem")
        );
        assert!(!net_env_map.contains_key("VESTA_TEST_SECRET"));
        assert!(!net_env_map.contains_key("OPENAI_API_KEY"));
        assert!(!net_env_map.contains_key("VESTA_DOWNLOADS_DIR"));

        let (downloads_res, network_res, bad_scope_err, empty_scope_err) = {
            let _lock = ENV_LOCK.lock().await;
            let _restore = EnvRestore(vec![(
                "VESTA_SANDBOX_TEST_SECRET",
                std::env::var_os("VESTA_SANDBOX_TEST_SECRET"),
            )]);
            // SAFETY: serialized under ENV_LOCK; key is private to this test.
            unsafe {
                std::env::set_var("VESTA_SANDBOX_TEST_SECRET", "top-secret");
            }

            let manifest_dl = python_manifest("env-dl", script, vec!["filesystem:downloads"]);
            let proc_dl = runtime
                .ensure_running(&manifest_dl, dir.path(), None, "en", &HashMap::new(), false)
                .await
                .unwrap();
            let dl_val = runtime
                .render(&proc_dl, "main", serde_json::json!({}), "en", false)
                .await
                .unwrap();
            runtime.stop_plugin("env-dl").await;

            let manifest_net = python_manifest("env-net", script, vec!["network"]);
            let proc_net = runtime
                .ensure_running(
                    &manifest_net,
                    dir.path(),
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                )
                .await
                .unwrap();
            let net_val = runtime
                .render(&proc_net, "main", serde_json::json!({}), "en", false)
                .await
                .unwrap();
            runtime.stop_plugin("env-net").await;

            let bad_manifest = python_manifest("env-bad", script, vec!["shell:exec"]);
            let bad_err = runtime
                .ensure_running(
                    &bad_manifest,
                    dir.path(),
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                )
                .await;
            let empty_manifest = python_manifest("env-empty", script, vec!["  "]);
            let empty_err = runtime
                .ensure_running(
                    &empty_manifest,
                    dir.path(),
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                )
                .await;

            (dl_val, net_val, bad_err, empty_err)
        };

        assert!(bad_scope_err.is_err());
        assert!(empty_scope_err.is_err());

        let dl_env = downloads_res["env"].as_object().unwrap();
        assert_eq!(dl_env["VESTA_PLUGIN_ID"], "env-dl");
        assert_eq!(dl_env["VESTA_API_VERSION"], "1");
        assert!(dl_env.contains_key("VESTA_DATA_DIR"));
        assert_eq!(dl_env["VESTA_PLUGIN_SCOPES"], "filesystem:downloads");
        assert!(dl_env.contains_key("VESTA_DOWNLOADS_DIR"));
        assert!(!dl_env.contains_key("VESTA_SANDBOX_TEST_SECRET"));
        assert_eq!(
            downloads_res["init"]["permissions"]["scopes"],
            serde_json::json!(["filesystem:downloads"])
        );
        assert!(downloads_res["init"]["permissions"]["downloadsDir"].is_string());

        let net_env = network_res["env"].as_object().unwrap();
        assert_eq!(net_env["VESTA_PLUGIN_SCOPES"], "network");
        assert!(!net_env.contains_key("VESTA_DOWNLOADS_DIR"));
        assert!(!net_env.contains_key("VESTA_SANDBOX_TEST_SECRET"));
        assert!(
            network_res["init"]["permissions"]
                .get("downloadsDir")
                .is_none()
        );
    }

    #[tokio::test]
    async fn bounded_stdout_rejects_oversized_rpc_line_without_oom() {
        let oversized = vec![b'a'; MAX_PLUGIN_RPC_LINE_BYTES + 1024];
        let mut reader = BufReader::new(std::io::Cursor::new(oversized));
        let err = read_bounded_rpc_line(&mut reader, MAX_PLUGIN_RPC_LINE_BYTES)
            .await
            .unwrap_err();
        assert!(err.contains("exceeded"));

        let dir = tempfile::tempdir().unwrap();
        let runtime = PluginRuntimeManager::new();
        let script = format!(
            r#"
import json, sys
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        print(json.dumps({{"jsonrpc": "2.0", "id": req["id"], "result": {{"ok": True}}}}), flush=True)
    elif m == "render":
        sys.stdout.write("a" * ({limit} + 4096) + "\n")
        sys.stdout.flush()
    elif m == "shutdown":
        break
"#,
            limit = MAX_PLUGIN_RPC_LINE_BYTES
        );
        let manifest = python_manifest("big-stdout", &script, vec![]);
        let proc = runtime
            .ensure_running(&manifest, dir.path(), None, "en", &HashMap::new(), false)
            .await
            .unwrap();
        let render_res = tokio::time::timeout(
            Duration::from_secs(5),
            runtime.render(&proc, "main", serde_json::json!({}), "en", false),
        )
        .await
        .expect("render must fail promptly instead of waiting 15s");
        assert!(render_res.is_err());
        runtime.stop_plugin("big-stdout").await;
    }

    #[tokio::test]
    async fn bounded_stderr_handles_oversized_line_without_oom() {
        let (mut writer, reader) = tokio::io::duplex(16 * 1024);
        let write_task = tokio::spawn(async move {
            let chunk = vec![b'x'; 64 * 1024];
            for _ in 0..16 {
                writer.write_all(&chunk).await.unwrap();
            }
            writer.write_all(b"\nnext line\n").await.unwrap();
            writer.shutdown().await.unwrap();
        });
        let mut buf_reader = BufReader::new(reader);
        let (first, truncated) =
            read_bounded_stderr_line(&mut buf_reader, MAX_PLUGIN_STDERR_LINE_BYTES)
                .await
                .unwrap()
                .unwrap();
        assert!(truncated);
        assert_eq!(first.len(), MAX_PLUGIN_STDERR_LINE_BYTES);
        let (second, truncated_second) =
            read_bounded_stderr_line(&mut buf_reader, MAX_PLUGIN_STDERR_LINE_BYTES)
                .await
                .unwrap()
                .unwrap();
        assert!(!truncated_second);
        assert_eq!(second, "next line");
        assert!(
            read_bounded_stderr_line(&mut buf_reader, MAX_PLUGIN_STDERR_LINE_BYTES)
                .await
                .unwrap()
                .is_none()
        );
        write_task.await.unwrap();

        let dir = tempfile::tempdir().unwrap();
        let runtime = PluginRuntimeManager::new();
        let script = r#"
import json, sys
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        sys.stderr.write("e" * (512 * 1024) + "\n")
        sys.stderr.flush()
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}), flush=True)
    elif m == "render":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"status": "ok"}}), flush=True)
    elif m == "shutdown":
        break
"#;
        let manifest = python_manifest("big-stderr", script, vec![]);
        let proc = runtime
            .ensure_running(&manifest, dir.path(), None, "en", &HashMap::new(), false)
            .await
            .unwrap();
        let rendered = runtime
            .render(&proc, "main", serde_json::json!({}), "en", false)
            .await
            .unwrap();
        assert_eq!(rendered["status"], "ok");
        runtime.stop_plugin("big-stderr").await;
    }

    #[tokio::test]
    async fn is_alive_returns_false_after_subprocess_exits_and_respawns() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = PluginRuntimeManager::new();
        let script = r#"
import json, os, sys
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"pid": os.getpid()}}), flush=True)
    elif m == "render":
        params = req.get("params", {}).get("params", {})
        if params.get("die"):
            sys.exit(0)
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"pid": os.getpid()}}), flush=True)
    elif m == "shutdown":
        break
"#;
        let manifest = python_manifest("respawn-plugin", script, vec![]);
        let proc1 = runtime
            .ensure_running(&manifest, dir.path(), None, "en", &HashMap::new(), false)
            .await
            .unwrap();
        let r1 = runtime
            .render(&proc1, "main", serde_json::json!({}), "en", false)
            .await
            .unwrap();
        assert!(proc1.is_alive());

        let _ = runtime
            .render(
                &proc1,
                "main",
                serde_json::json!({"die": true}),
                "en",
                false,
            )
            .await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while proc1.is_alive() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(!proc1.is_alive());

        let proc2 = runtime
            .ensure_running(&manifest, dir.path(), None, "en", &HashMap::new(), false)
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&proc1, &proc2));
        assert!(proc2.is_alive());
        let r2 = runtime
            .render(&proc2, "main", serde_json::json!({}), "en", false)
            .await
            .unwrap();
        assert_ne!(r1["pid"], r2["pid"]);
        runtime.stop_plugin("respawn-plugin").await;
    }

    #[tokio::test]
    async fn initialize_failure_kills_child_and_does_not_cache_broken_process() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = PluginRuntimeManager::new();

        // Subcase A: immediate exit / RPC error on initialize
        let fail_script = r#"
import json, sys
line = sys.stdin.readline()
req = json.loads(line)
print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32000, "message": "init boom"}}), flush=True)
sys.exit(1)
"#;
        let fail_manifest = python_manifest("init-error", fail_script, vec![]);
        let err = runtime
            .ensure_running(
                &fail_manifest,
                dir.path(),
                None,
                "en",
                &HashMap::new(),
                false,
            )
            .await;
        assert!(err.is_err());
        assert!(!runtime.processes.lock().await.contains_key("init-error"));

        // Subcase B: timeout on initialize kills and reaps child
        let pid_file = dir.path().join("slow.pid");
        let sleep_script = format!(
            r#"
import os, time
with open({pid_path:?}, "w") as f:
    f.write(str(os.getpid()))
time.sleep(5)
"#,
            pid_path = pid_file.to_string_lossy()
        );
        let timeout_manifest = python_manifest("init-timeout", &sleep_script, vec![]);
        let timeout_err = runtime
            .ensure_running_with_init_timeout(
                &timeout_manifest,
                dir.path(),
                None,
                "en",
                &HashMap::new(),
                false,
                Duration::from_millis(200),
            )
            .await;
        assert!(timeout_err.is_err());
        assert!(!runtime.processes.lock().await.contains_key("init-timeout"));

        if let Ok(pid_text) = std::fs::read_to_string(&pid_file)
            && let Ok(pid) = pid_text.trim().parse::<u32>()
        {
            #[cfg(target_os = "linux")]
            assert!(
                !Path::new(&format!("/proc/{pid}")).exists(),
                "timed out plugin pid {pid} must be killed and reaped"
            );
        }
    }

    #[tokio::test]
    async fn processes_lock_is_not_held_during_slow_initialize() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Arc::new(PluginRuntimeManager::new());

        let slow_script = r#"
import json, sys, time
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        time.sleep(0.45)
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}), flush=True)
    elif m == "shutdown":
        break
"#;
        let fast_script = r#"
import json, sys
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        print(json.dumps({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": True}}), flush=True)
    elif m == "shutdown":
        break
"#;

        let slow_manifest = python_manifest("slow-plugin", slow_script, vec![]);
        let fast_manifest = python_manifest("fast-plugin", fast_script, vec![]);

        let slow_done = Arc::new(AtomicBool::new(false));
        let slow_done_clone = slow_done.clone();
        let runtime_slow = runtime.clone();
        let dir_slow = dir.path().to_path_buf();
        let slow_task = tokio::spawn(async move {
            let res = runtime_slow
                .ensure_running(
                    &slow_manifest,
                    &dir_slow,
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                )
                .await;
            slow_done_clone.store(true, Ordering::SeqCst);
            res
        });

        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(
            runtime.processes.try_lock().is_ok(),
            "processes mutex must not be held during slow initialize"
        );

        let fast_start = tokio::time::Instant::now();
        let fast_proc = runtime
            .ensure_running(
                &fast_manifest,
                dir.path(),
                None,
                "en",
                &HashMap::new(),
                false,
            )
            .await
            .unwrap();
        let fast_elapsed = fast_start.elapsed();
        assert!(fast_proc.is_alive());
        assert!(
            fast_elapsed < Duration::from_millis(300),
            "fast plugin took too long ({fast_elapsed:?}); slow init blocked it"
        );
        assert!(
            !slow_done.load(Ordering::SeqCst),
            "fast plugin should finish before slow plugin completes initialize"
        );

        let slow_proc = slow_task.await.unwrap().unwrap();
        assert!(slow_proc.is_alive());

        // Concurrent ensure_running for the same slow plugin ID spawns only once
        let count_file = dir.path().join("spawns.txt");
        let dedup_script = format!(
            r#"
import json, os, sys, time
with open({count_path:?}, "a") as f:
    f.write(str(os.getpid()) + "\n")
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        time.sleep(0.25)
        print(json.dumps({{"jsonrpc": "2.0", "id": req["id"], "result": {{"ok": True}}}}), flush=True)
    elif m == "shutdown":
        break
"#,
            count_path = count_file.to_string_lossy()
        );
        let dedup_manifest = python_manifest("dedup-plugin", &dedup_script, vec![]);
        let empty_settings = HashMap::new();
        let (r1, r2) = tokio::join!(
            runtime.ensure_running(
                &dedup_manifest,
                dir.path(),
                None,
                "en",
                &empty_settings,
                false
            ),
            runtime.ensure_running(
                &dedup_manifest,
                dir.path(),
                None,
                "en",
                &empty_settings,
                false
            )
        );
        let p1 = r1.unwrap();
        let p2 = r2.unwrap();
        assert!(Arc::ptr_eq(&p1, &p2));
        let spawns = std::fs::read_to_string(&count_file).unwrap();
        assert_eq!(spawns.lines().count(), 1);

        runtime.stop_plugin("fast-plugin").await;
        runtime.stop_plugin("slow-plugin").await;
        runtime.stop_plugin("dedup-plugin").await;
    }

    #[tokio::test]
    async fn stop_plugin_wakes_slow_initialize_immediately_via_stop_notify() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Arc::new(PluginRuntimeManager::new());

        let pid_file = dir.path().join("notify.pid");
        let ready_file = dir.path().join("notify.ready");
        let script = format!(
            r#"
import json, os, sys, time
with open({pid_path:?}, "w") as f:
    f.write(str(os.getpid()))
    f.flush()
for line in sys.stdin:
    req = json.loads(line)
    if req.get("method") == "initialize":
        with open({ready_path:?}, "w") as f:
            f.write("ready")
            f.flush()
        time.sleep(10)
        break
"#,
            pid_path = pid_file.to_string_lossy(),
            ready_path = ready_file.to_string_lossy(),
        );
        let manifest = python_manifest("stop-notify-immediate", &script, vec![]);

        let runtime_init = runtime.clone();
        let dir_init = dir.path().to_path_buf();
        let start = tokio::time::Instant::now();
        let init_task = tokio::spawn(async move {
            runtime_init
                .ensure_running_with_init_timeout(
                    &manifest,
                    &dir_init,
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                    Duration::from_secs(10),
                )
                .await
        });

        let wait_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !ready_file.exists() && tokio::time::Instant::now() < wait_deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(ready_file.exists(), "plugin must reach initialize");

        runtime.stop_plugin("stop-notify-immediate").await;
        let init_res = tokio::time::timeout(Duration::from_secs(3), init_task)
            .await
            .expect("stop_plugin must wake initialize immediately instead of waiting 10s")
            .unwrap();
        let elapsed = start.elapsed();

        assert!(init_res.is_err());
        assert!(
            elapsed < Duration::from_secs(3),
            "stop_notify wakeup took too long: {elapsed:?}"
        );
        assert!(
            !runtime
                .processes
                .lock()
                .await
                .contains_key("stop-notify-immediate")
        );

        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        #[cfg(target_os = "linux")]
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "cancelled plugin pid {pid} must be killed and reaped"
        );
    }

    #[tokio::test]
    async fn stop_plugin_cancelling_as_initialize_finishes_does_not_insert_or_leak() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Arc::new(PluginRuntimeManager::new());

        let pid_file = dir.path().join("boundary.pid");
        let ready_file = dir.path().join("boundary.ready");
        let go_file = dir.path().join("boundary.go");
        let script = format!(
            r#"
import json, os, sys, time
with open({pid_path:?}, "w") as f:
    f.write(str(os.getpid()))
    f.flush()
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    if m == "initialize":
        with open({ready_path:?}, "w") as f:
            f.write("ready")
            f.flush()
        while not os.path.exists({go_path:?}):
            time.sleep(0.005)
        print(json.dumps({{"jsonrpc": "2.0", "id": req["id"], "result": {{"ok": True}}}}), flush=True)
    elif m == "shutdown":
        print(json.dumps({{"jsonrpc": "2.0", "id": req["id"], "result": None}}), flush=True)
        break
"#,
            pid_path = pid_file.to_string_lossy(),
            ready_path = ready_file.to_string_lossy(),
            go_path = go_file.to_string_lossy(),
        );
        let manifest = python_manifest("stop-at-init-boundary", &script, vec![]);

        let runtime_init = runtime.clone();
        let dir_init = dir.path().to_path_buf();
        let manifest_init = manifest.clone();
        let init_task = tokio::spawn(async move {
            runtime_init
                .ensure_running_with_init_timeout(
                    &manifest_init,
                    &dir_init,
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                    Duration::from_secs(5),
                )
                .await
        });

        let wait_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !ready_file.exists() && tokio::time::Instant::now() < wait_deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(ready_file.exists(), "plugin must reach initialize");

        // Hold `processes` lock so `ensure_running` completes `initialize` and parks right on
        // `self.processes.lock().await` while `stop_plugin` increments `slot.generation`.
        let procs_guard = runtime.processes.lock().await;
        std::fs::write(&go_file, "go").unwrap();
        tokio::time::sleep(Duration::from_millis(60)).await;

        let runtime_stop = runtime.clone();
        let stop_task = tokio::spawn(async move {
            runtime_stop.stop_plugin("stop-at-init-boundary").await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(procs_guard);

        let (init_res, stop_res) = tokio::join!(init_task, stop_task);
        stop_res.unwrap();
        let err = match init_res.unwrap() {
            Ok(_) => {
                panic!("ensure_running must not succeed after stop_plugin incremented generation")
            }
            Err(err) => err,
        };
        assert!(
            err.contains("stopped during initialization")
                || err.contains("cancelled by stop_plugin"),
            "unexpected error message: {err}"
        );
        assert!(
            !runtime
                .processes
                .lock()
                .await
                .contains_key("stop-at-init-boundary"),
            "stopped plugin must never be inserted into processes"
        );

        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        #[cfg(target_os = "linux")]
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "stopped plugin pid {pid} must be killed and reaped"
        );

        // Concurrent race without holding `processes`: plugin must never remain cached or alive.
        let _ = std::fs::remove_file(&ready_file);
        let _ = std::fs::remove_file(&go_file);
        let _ = std::fs::remove_file(&pid_file);

        let runtime_init2 = runtime.clone();
        let dir_init2 = dir.path().to_path_buf();
        let init_task2 = tokio::spawn(async move {
            runtime_init2
                .ensure_running_with_init_timeout(
                    &manifest,
                    &dir_init2,
                    None,
                    "en",
                    &HashMap::new(),
                    false,
                    Duration::from_secs(5),
                )
                .await
        });

        let wait_deadline2 = tokio::time::Instant::now() + Duration::from_secs(5);
        while !ready_file.exists() && tokio::time::Instant::now() < wait_deadline2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(ready_file.exists(), "plugin must reach initialize on retry");

        std::fs::write(&go_file, "go").unwrap();
        let runtime_stop2 = runtime.clone();
        let (_, _) = tokio::join!(
            init_task2,
            runtime_stop2.stop_plugin("stop-at-init-boundary")
        );

        assert!(
            !runtime
                .processes
                .lock()
                .await
                .contains_key("stop-at-init-boundary"),
            "concurrently stopped plugin must not remain in processes"
        );
        let pid2: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        #[cfg(target_os = "linux")]
        assert!(
            !Path::new(&format!("/proc/{pid2}")).exists(),
            "concurrently stopped plugin pid {pid2} must be killed and reaped"
        );
    }
}

use crate::plugins::manifest::PluginManifest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc, oneshot};

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
    child_handle: Arc<Mutex<Option<tokio::process::Child>>>,
}

impl ActivePluginProcess {
    pub fn is_alive(&self) -> bool {
        !self.stdin_tx.is_closed()
    }
}

pub struct PluginRuntimeManager {
    processes: Arc<Mutex<HashMap<String, Arc<ActivePluginProcess>>>>,
}

impl Default for PluginRuntimeManager {
    fn default() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl PluginRuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops and removes any active process for the given plugin.
    pub async fn stop_plugin(&self, plugin_id: &str) {
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

            let mut child_guard = proc.child_handle.lock().await;
            if let Some(mut child) = child_guard.take() {
                let _ = child.kill().await;
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
        shared_credentials: &BTreeMap<String, String>,
    ) -> Result<Arc<ActivePluginProcess>, String> {
        let mut procs = self.processes.lock().await;

        if let Some(existing) = procs.get(&manifest.id)
            && existing.is_alive()
        {
            return Ok(existing.clone());
        }

        // Spawn child process
        let command = manifest.resolve_command();
        if command.is_empty() {
            return Err("Plugin manifest specifies empty entry command".to_string());
        }

        let executable = &command[0];
        let args = &command[1..];

        let data_dir = plugin_dir.join("data");
        let _ = tokio::fs::create_dir_all(&data_dir).await;

        let mut cmd = tokio::process::Command::new(executable);
        cmd.args(args)
            .current_dir(plugin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("VESTA_PLUGIN_ID", &manifest.id)
            .env("VESTA_API_VERSION", "1")
            .env("VESTA_DATA_DIR", data_dir.to_string_lossy().as_ref());

        let mut child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn plugin '{}' ({}): {e}",
                manifest.id, executable
            )
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture stdin for plugin process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout for plugin process".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr for plugin process".to_string())?;

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(64);
        let pending = Arc::new(Mutex::new(HashMap::<
            u64,
            oneshot::Sender<Result<serde_json::Value, String>>,
        >::new()));
        let latest_values = Arc::new(Mutex::new(HashMap::<String, serde_json::Value>::new()));

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

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
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
                                let _ = tx
                                    .send(Err(format!("[RPC Error {}] {}", err.code, err.message)));
                            } else {
                                let _ = tx.send(Ok(msg.result.unwrap_or(serde_json::Value::Null)));
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
        });

        // Stderr logger task
        let plugin_id_err = manifest.id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                eprintln!("[plugin:{plugin_id_err}:stderr] {line}");
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
            child_handle: Arc::new(Mutex::new(Some(child))),
        });

        // Initialize plugin
        let init_params = serde_json::json!({
            "apiVersion": 1,
            "dataDir": data_dir.to_string_lossy(),
            "locale": locale,
            "settings": settings,
            "expertMode": expert_mode,
            "sharedCredentials": shared_credentials,
        });

        let init_res =
            Self::send_raw_request(&active, "initialize", init_params, Duration::from_secs(10))
                .await;
        if let Err(e) = init_res {
            eprintln!(
                "Warning: Plugin '{}' initialize returned error: {e}",
                manifest.id
            );
        } else {
            active.initialized.store(true, Ordering::SeqCst);
        }

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
        shared_credentials: &BTreeMap<String, String>,
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
                "sharedCredentials": shared_credentials,
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
        shared_credentials: &BTreeMap<String, String>,
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
                "sharedCredentials": shared_credentials,
            }),
            Duration::from_secs(30),
        )
        .await
    }
}

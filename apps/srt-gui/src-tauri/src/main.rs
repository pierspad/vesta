// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

mod commands;
mod state;

use axum::{
    extract::{Query, Request},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceExt;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeFile,
};

#[derive(serde::Deserialize)]
struct MediaParams {
    path: String,
}

struct MediaServerPort(u16);

#[tauri::command]
fn get_media_server_port(port: tauri::State<MediaServerPort>) -> u16 {
    port.0
}

async fn media_handler(
    Query(params): Query<MediaParams>,
    req: Request,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    ServeFile::new(&params.path)
        .oneshot(req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

use commands::auto_sync::*;
use commands::flashcards::*;
use commands::info::*;
use commands::sync::*;
use commands::transcribe::*;
use commands::translate::*;
use state::{
    AppFlashcardState, AppSyncState, AppTranscribeState, AppTranslateState, FlashcardState,
    SyncState, TranscribeState, TranslateState,
};

/// Determina il MIME type in base all'estensione
fn mime_from_ext(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" | "wave" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "wma" => "audio/x-ms-wma",
        _ => "application/octet-stream",
    }
}

fn main() {
    // Fix blurry rendering on Linux (WebKitGTK DMABUF renderer issue)
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // Prevent WebKitWebProcess crash when gst-plugins-good is not installed
        // (missing autoaudiosink element causes the app to go grey/unresponsive)
        std::env::set_var("WEBKIT_DISABLE_MEDIA_STREAM", "1");
        // Disable GStreamer audio/video sinks entirely to prevent
        // "GStreamer element autoaudiosink not found" errors on drag-drop
        std::env::set_var(
            "GST_PLUGIN_FEATURE_RANK",
            "autoaudiosink:0,autovideosink:0,pulsesink:0,alsasink:0",
        );
        // Prevent WebKit from using GStreamer for content sniffing on dropped files
        std::env::set_var("GST_REGISTRY_UPDATE", "no");
    }

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();

    tauri::async_runtime::spawn(async move {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/media", get(media_handler))
            .layer(cors);

        let tokio_listener = tokio::net::TcpListener::from_std(listener).unwrap();
        axum::serve(tokio_listener, app).await.unwrap();
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        // Protocollo custom per lo streaming video con supporto Range requests
        .register_asynchronous_uri_scheme_protocol("stream", |_ctx, request, responder| {
            std::thread::spawn(move || {
                let uri = request.uri().to_string();
                // URI formato: stream://localhost/<encoded_path>
                let path = uri
                    .strip_prefix("stream://localhost/")
                    .or_else(|| uri.strip_prefix("stream://localhost"))
                    .unwrap_or("");
                let path = urlencoding::decode(path).unwrap_or_else(|_| path.into());
                let path = path.to_string();

                eprintln!("[stream] Request URI: {}", uri);
                eprintln!("[stream] Decoded path: '{}'", path);

                // Verifica che il file esista
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("[stream] File not found or inaccessible: '{}' - Error: {}", path, e);
                        let resp = tauri::http::Response::builder()
                            .status(404)
                            .header("Content-Type", "text/plain")
                            .body(format!("File not found: {} - {}", path, e).into_bytes())
                            .unwrap();
                        responder.respond(resp);
                        return;
                    }
                };

                let file_size = metadata.len();
                let mime = mime_from_ext(&path);
                eprintln!("[stream] File: '{}', size: {} bytes, mime: {}", path, file_size, mime);

                // Parse Range header
                let range_header = request.headers().get("range").and_then(|v| v.to_str().ok());

                if let Some(range_str) = range_header {
                    eprintln!("[stream] Range request: {}", range_str);
                    // Parse "bytes=START-END" or "bytes=START-"
                    let range_str = range_str.trim_start_matches("bytes=");
                    let parts: Vec<&str> = range_str.split('-').collect();
                    let start: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let end: u64 = parts
                        .get(1)
                        .and_then(|s| if s.is_empty() { None } else { s.parse().ok() })
                        .unwrap_or(file_size - 1)
                        .min(file_size - 1);

                    let chunk_size = end - start + 1;

                    // Leggi il chunk richiesto
                    let mut file = match fs::File::open(&path) {
                        Ok(f) => f,
                        Err(_) => {
                            let resp = tauri::http::Response::builder()
                                .status(500)
                                .body(b"Failed to open file".to_vec())
                                .unwrap();
                            responder.respond(resp);
                            return;
                        }
                    };

                    if file.seek(SeekFrom::Start(start)).is_err() {
                        let resp = tauri::http::Response::builder()
                            .status(500)
                            .body(b"Seek failed".to_vec())
                            .unwrap();
                        responder.respond(resp);
                        return;
                    }

                    // Limita chunk a 4MB per evitare uso eccessivo di memoria
                    let max_chunk = 4 * 1024 * 1024u64;
                    let read_size = chunk_size.min(max_chunk) as usize;
                    let mut buf = vec![0u8; read_size];
                    let bytes_read = match file.read(&mut buf) {
                        Ok(n) => n,
                        Err(_) => {
                            let resp = tauri::http::Response::builder()
                                .status(500)
                                .body(b"Read failed".to_vec())
                                .unwrap();
                            responder.respond(resp);
                            return;
                        }
                    };
                    buf.truncate(bytes_read);

                    let actual_end = start + bytes_read as u64 - 1;

                    eprintln!("[stream] Range response: bytes {}-{}/{}, chunk={} bytes", start, actual_end, file_size, bytes_read);

                    let resp = tauri::http::Response::builder()
                        .status(206)
                        .header("Content-Type", mime)
                        .header("Accept-Ranges", "bytes")
                        .header("Content-Range", format!("bytes {}-{}/{}", start, actual_end, file_size))
                        .header("Content-Length", bytes_read.to_string())
                        .body(buf)
                        .unwrap();
                    responder.respond(resp);
                } else {
                    // Nessun Range: restituisci header con Accept-Ranges 
                    // ma non l'intero file (potrebbe essere enorme).
                    // Rispondiamo con 206 Partial Content per i primi bytes,
                    // così il media player sa la dimensione totale e può fare Range requests.
                    let mut file = match fs::File::open(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("[stream] Failed to open file '{}': {}", path, e);
                            let resp = tauri::http::Response::builder()
                                .status(500)
                                .header("Content-Type", "text/plain")
                                .body(format!("Failed to open file: {}", e).into_bytes())
                                .unwrap();
                            responder.respond(resp);
                            return;
                        }
                    };

                    let max_initial = 2 * 1024 * 1024u64; // 2MB initial read
                    let read_size = (file_size).min(max_initial) as usize;
                    let mut buf = vec![0u8; read_size];
                    let bytes_read = file.read(&mut buf).unwrap_or(0);
                    buf.truncate(bytes_read);

                    eprintln!("[stream] Serving initial response for '{}': mime={}, file_size={}, bytes_sent={}", path, mime, file_size, bytes_read);

                    if (bytes_read as u64) < file_size {
                        // File più grande del chunk iniziale: rispondi con 206 Partial Content
                        let actual_end = bytes_read as u64 - 1;
                        let resp = tauri::http::Response::builder()
                            .status(206)
                            .header("Content-Type", mime)
                            .header("Accept-Ranges", "bytes")
                            .header("Content-Range", format!("bytes 0-{}/{}", actual_end, file_size))
                            .header("Content-Length", bytes_read.to_string())
                            .body(buf)
                            .unwrap();
                        responder.respond(resp);
                    } else {
                        // File piccolo: restituisci tutto con 200
                        let resp = tauri::http::Response::builder()
                            .status(200)
                            .header("Content-Type", mime)
                            .header("Accept-Ranges", "bytes")
                            .header("Content-Length", bytes_read.to_string())
                            .body(buf)
                            .unwrap();
                        responder.respond(resp);
                    }
                }
            });
        })
        .manage(Mutex::new(SyncState::default()) as AppSyncState)
        .manage(Mutex::new(TranslateState::default()) as AppTranslateState)
        .manage(Mutex::new(FlashcardState::default()) as AppFlashcardState)
        .manage(Mutex::new(TranscribeState::default()) as AppTranscribeState)
        .manage(MediaServerPort(port))
        .setup(|app| {
            // On Linux, prevent WebKit from navigating to file:// URLs when files
            // are drag-dropped onto the webview. Without this, WebKit tries to
            // load video files via GStreamer, causing a crash when gst-plugins are
            // missing (autoaudiosink not found).
            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|wv| {
                        #[allow(deprecated)]
                        {
                        use webkit2gtk::WebViewExt;
                        use webkit2gtk::NavigationPolicyDecision;
                        use webkit2gtk::NavigationPolicyDecisionExt;
                        use webkit2gtk::PolicyDecisionExt;
                        use webkit2gtk::PolicyDecisionType;
                        use webkit2gtk::URIRequestExt;
                        use webkit2gtk::glib::{Cast, ObjectExt};
                        let webview = wv.inner();
                        let wk: &webkit2gtk::WebView = webview.as_ref();

                        // 1) Block file:// navigations (belt-and-suspenders with JS preventDefault)
                        wk.connect_decide_policy(|_wv, decision, decision_type| {
                            if decision_type == PolicyDecisionType::NavigationAction
                                || decision_type == PolicyDecisionType::NewWindowAction
                            {
                                if let Some(nav) = decision.downcast_ref::<NavigationPolicyDecision>() {
                                    if let Some(request) = NavigationPolicyDecisionExt::request(nav) {
                                        if let Some(uri) = URIRequestExt::uri(&request) {
                                            if uri.starts_with("file://") {
                                                decision.ignore();
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                            false
                        });

                        // 2) Prevent WebKit from processing dropped file data via GStreamer.
                        //    wry (Tauri's webview layer) connects its own drag-data-received
                        //    handler FIRST (during webview construction) which extracts URIs
                        //    and emits Tauri's DragDrop events.  Our handler fires AFTER wry's
                        //    but BEFORE WebKit's class handler.  By stopping the signal here,
                        //    WebKit's default handler never runs, so the WebProcess never
                        //    receives the dropped data and never invokes GStreamer — avoiding
                        //    the "autoaudiosink not found" crash entirely.
                        let wk_obj: webkit2gtk::glib::Object = wk.clone().upcast();
                        wk_obj.connect_local("drag-data-received", false, |values| {
                            if let Ok(widget) = values[0].get::<webkit2gtk::glib::Object>() {
                                widget.stop_signal_emission_by_name("drag-data-received");
                            }
                            None
                        });
                        }
                    });
                }
            }

            // CLI BENCHMARK MODE
            let args: Vec<String> = std::env::args().collect();
            if args.len() >= 6 && args[1] == "--benchmark" {
                let app_handle = app.handle().clone();
                let sub1 = args[2].clone();
                let sub2 = args[3].clone();
                let video = args[4].clone();
                let out_dir = args[5].clone();
                let export_fmt = if args.len() >= 7 { args[6].clone() } else { "tsv".to_string() };

                tauri::async_runtime::spawn(async move {
                    use std::time::Instant;
                    use crate::commands::flashcards::types::{FlashcardConfig, SubtitleFilters, ContextConfig, OutputFields};
                    use tauri::Manager;

                    let has_audio = {
                        let output = std::process::Command::new("ffprobe")
                            .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0", &video])
                            .output()
                            .expect("failed to execute ffprobe");
                        String::from_utf8_lossy(&output.stdout).contains("audio")
                    };

                    let config = FlashcardConfig {
                        target_subs_path: sub1,
                        native_subs_path: Some(sub2),
                        video_path: Some(video.clone()),
                        audio_path: if has_audio { Some(video) } else { None },
                        output_dir: out_dir,
                        use_timings_from: "target".to_string(),
                        span_start_ms: None,
                        span_end_ms: None,
                        time_shift_target_ms: 0,
                        time_shift_native_ms: 0,
                        filters: SubtitleFilters {
                            include_words: None,
                            exclude_words: None,
                            exclude_duplicates_subs1: false,
                            exclude_duplicates_subs2: false,
                            min_chars: None,
                            max_chars: None,
                            min_duration_ms: None,
                            max_duration_ms: None,
                            exclude_styled: false,
                            actor_filter: None,
                            only_cjk: false,
                            remove_no_match: false,
                        },
                        context: ContextConfig { leading: 0, trailing: 0, max_gap_seconds: 0.0 },
                        combine_sentences: false,
                        continuation_chars: "".to_string(),
                        generate_audio: has_audio,
                        audio_bitrate: 128,
                        audio_track_index: None,
                        normalize_audio: false,
                        audio_pad_start_ms: 0,
                        audio_pad_end_ms: 0,
                        generate_snapshots: true,
                        snapshot_width: 240,
                        snapshot_height: 160,
                        crop_bottom: 0,
                        generate_video_clips: true,
                        video_codec: "h264".to_string(),
                        h264_preset: "ultrafast".to_string(),
                        video_bitrate: 1000,
                        video_audio_bitrate: 128,
                        video_pad_start_ms: 0,
                        video_pad_end_ms: 0,
                        deck_name: "BenchmarkDeck".to_string(),
                        episode_number: 1,
                        export_format: Some(export_fmt),
                        note_type_name: None,
                        output_fields: OutputFields {
                            include_tag: true,
                            include_sequence: true,
                            include_audio: true,
                            include_snapshot: true,
                            include_video: true,
                            include_subs1: true,
                            include_subs2: true,
                        },
                        cpu_cores: None,
                        card_front_html: None,
                        card_back_html: None,
                        card_css: None,
                    };

                    let state = app_handle.state::<crate::AppFlashcardState>();

                    let start = Instant::now();
                    let res = crate::commands::flashcards::commands::flashcard_generate(
                        app_handle.clone(),
                        state,
                        config
                    ).await;
                    let duration = start.elapsed();

                    match res {
                        Ok(_) => println!("VESTA_BENCHMARK_SUCCESS: {} ms", duration.as_millis()),
                        Err(e) => println!("VESTA_BENCHMARK_ERROR: {}", e),
                    }
                    std::process::exit(0);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_media_server_port,
            // Comandi app info
            get_app_info,
            // Comandi traduzione
            set_api_config,
            load_srt_for_translate,
            start_translation,
            cancel_translation,
            get_latest_translated_subtitles,
            // Comandi sincronizzazione
            sync_load_srt,
            sync_suggest_media_for_srt,
            sync_suggest_companion_subtitle_for_srt,
            sync_set_video,
            sync_get_status,
            sync_get_subtitles,
            sync_get_subtitles_range,
            sync_get_subtitle,
            sync_find_subtitle_at_time,
            sync_find_nearest_subtitle,
            sync_add_anchor,
            sync_remove_anchor,
            sync_get_anchors,
            sync_suggest_next,
            sync_set_strategy,
            sync_save_file,
            sync_save_session,
            sync_load_session,
            sync_reset,
            // Comandi flashcard
            flashcard_load_subs,
            flashcard_preview,
            flashcard_generate,
            flashcard_cancel,
            flashcard_list_audio_tracks,
            flashcard_check_deps,
            flashcard_download_ffmpeg,
            flashcard_check_dir_exists,
            flashcard_get_cpu_count,
            // Comandi trascrizione
            transcribe_check_backends,
            transcribe_list_models,
            transcribe_download_model,
            transcribe_uninstall_model,
            transcribe_start,
            transcribe_cancel,
            transcribe_check_file_exists,
            // Comandi auto-sync
            sync_auto_sync,
            sync_cancel_auto_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

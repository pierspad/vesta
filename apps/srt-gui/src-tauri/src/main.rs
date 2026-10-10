#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

mod commands;
mod media_range;
mod media_stream;
mod state;

use axum::{
    Router,
    extract::{Query, Request},
    response::IntoResponse,
    routing::get,
};
use tower::ServiceExt;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeFile,
};

#[derive(serde::Deserialize)]
struct MediaParams {
    path: String,
    #[serde(default)]
    token: String,
}

#[derive(Clone)]
struct MediaServerInfo {
    port: u16,
    token: String,
}

fn generate_media_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut token = String::with_capacity(32);
    for _ in 0..2 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::UNIX_EPOCH
                .elapsed()
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        token.push_str(&format!("{:016x}", h.finish()));
    }
    token
}

#[tauri::command]
fn get_media_server_info(info: tauri::State<MediaServerInfo>) -> (u16, String) {
    (info.port, info.token.clone())
}

async fn media_handler(
    axum::extract::State(expected_token): axum::extract::State<String>,
    Query(params): Query<MediaParams>,
    req: Request,
) -> Result<impl IntoResponse, axum::http::StatusCode> {
    if params.token != expected_token {
        return Err(axum::http::StatusCode::FORBIDDEN);
    }
    ServeFile::new(&params.path)
        .oneshot(req)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

use commands::auto_sync::*;
use commands::config::*;
use commands::experimental::*;
use commands::extract::*;
use commands::flashcards::*;
use commands::info::*;
use commands::net::*;
use commands::refine::*;
use commands::support_logs::*;
use commands::sync::*;
use commands::transcribe::*;
use commands::translate::*;
use commands::updates::*;
use state::{
    AppFlashcardState, AppRefineState, AppSyncState, AppTranscribeState, AppTranslateState,
    FlashcardState, RefineState, SyncState, TranscribeState, TranslateState,
};

fn main() {
    // Fix blurry rendering on Linux (WebKitGTK DMABUF renderer issue)
    #[cfg(target_os = "linux")]
    // SAFETY: this runs at the very top of `main`, single-threaded, before any other
    // thread (ours or WebKit's) is spawned or reads the environment, so there's no
    // concurrent-access race — the precondition `std::env::set_var` requires since it
    // became `unsafe` (env vars aren't thread-safe on all platforms).
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

        std::env::set_var("WEBKIT_DISABLE_MEDIA_STREAM", "1");
    }

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let media_token = generate_media_token();

    {
        let media_token = media_token.clone();
        tauri::async_runtime::spawn(async move {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route("/media", get(media_handler))
                .with_state(media_token)
                .layer(cors);

            let tokio_listener = tokio::net::TcpListener::from_std(listener).unwrap();
            axum::serve(tokio_listener, app).await.unwrap();
        });
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .register_asynchronous_uri_scheme_protocol("stream", |_ctx, request, responder| {
            std::thread::spawn(move || responder.respond(media_stream::response(&request)));
        })
        .manage(Mutex::new(SyncState::default()) as AppSyncState)
        .manage(Mutex::new(TranslateState::default()) as AppTranslateState)
        .manage(Mutex::new(FlashcardState::default()) as AppFlashcardState)
        .manage(Mutex::new(TranscribeState::default()) as AppTranscribeState)
        .manage(Mutex::new(RefineState::default()) as AppRefineState)
        .manage(commands::config::ConfigState::default())
        .manage(MediaServerInfo {
            port,
            token: media_token,
        })
        .setup(|app| {
            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|wv| {
                        #[allow(deprecated)]
                        {
                            use webkit2gtk::NavigationPolicyDecision;
                            use webkit2gtk::NavigationPolicyDecisionExt;
                            use webkit2gtk::PolicyDecisionExt;
                            use webkit2gtk::PolicyDecisionType;
                            use webkit2gtk::URIRequestExt;
                            use webkit2gtk::WebViewExt;
                            use webkit2gtk::glib::Cast;
                            let webview = wv.inner();
                            let wk: &webkit2gtk::WebView = webview.as_ref();

                            wk.connect_decide_policy(|_wv, decision, decision_type| {
                                if (decision_type == PolicyDecisionType::NavigationAction
                                    || decision_type == PolicyDecisionType::NewWindowAction)
                                    && let Some(nav) =
                                        decision.downcast_ref::<NavigationPolicyDecision>()
                                    && let Some(request) = NavigationPolicyDecisionExt::request(nav)
                                    && let Some(uri) = URIRequestExt::uri(&request)
                                    && uri.starts_with("file://")
                                {
                                    decision.ignore();
                                    return true;
                                }
                                false
                            });
                        }
                    });
                }
            }

            let args: Vec<String> = std::env::args().collect();
            if args.len() >= 6 && args[1] == "--benchmark" {
                let app_handle = app.handle().clone();
                let sub1 = args[2].clone();
                let sub2 = args[3].clone();
                let video = args[4].clone();
                let out_dir = args[5].clone();
                let export_fmt = if args.len() >= 7 {
                    args[6].clone()
                } else {
                    "tsv".to_string()
                };

                tauri::async_runtime::spawn(async move {
                    use crate::commands::flashcards::types::{FlashcardConfig, video_has_audio};
                    use std::time::Instant;
                    use tauri::Manager;

                    let config = FlashcardConfig::benchmark(
                        sub1,
                        sub2,
                        video.clone(),
                        out_dir,
                        export_fmt,
                        video_has_audio("ffprobe", &video),
                        None,
                    );

                    let state = app_handle.state::<crate::AppFlashcardState>();

                    let start = Instant::now();
                    let res = crate::commands::flashcards::commands::flashcard_generate(
                        app_handle.clone(),
                        state,
                        config,
                    )
                    .await;
                    let duration = start.elapsed();

                    match res {
                        Ok(_) => println!("vesta_BENCHMARK_SUCCESS: {} ms", duration.as_millis()),
                        Err(e) => println!("vesta_BENCHMARK_ERROR: {}", e),
                    }
                    std::process::exit(0);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_media_server_info,
            embedded_subtitle_tracks,
            existing_subtitle_outputs,
            extract_embedded_subtitle,
            preview_embedded_subtitle,
            open_output_path,
            get_app_info,
            get_update_installation,
            install_release_update,
            get_system_diagnostics,
            support_log_status,
            support_log_start,
            support_log_append,
            support_log_stop,
            support_log_export,
            read_subtitle_file,
            http_fetch,
            load_srt_for_translate,
            suggest_translation_context,
            start_translation,
            cancel_translation,
            get_latest_translated_subtitles,
            sync_load_srt,
            sync_suggest_media_for_srt,
            sync_suggest_companion_subtitle_for_srt,
            sync_suggest_subtitles_for_media,
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
            sync_prepare_media_for_playback,
            flashcard_load_subs,
            flashcard_preview,
            flashcard_preview_audio,
            flashcard_preview_snapshot,
            flashcard_parse_subtitles,
            flashcard_generate,
            flashcard_merge_apkg,
            flashcard_cancel,
            flashcard_list_audio_tracks,
            flashcard_check_deps,
            flashcard_download_ffmpeg,
            flashcard_check_dir_exists,
            flashcard_get_cpu_count,
            flashcard_get_total_memory_mb,
            flashcard_list_fonts,
            flashcard_check_language_font,
            flashcard_download_font,
            flashcard_delete_font,
            save_temp_subtitles,
            transcribe_check_backends,
            transcribe_list_models,
            transcribe_download_model,
            transcribe_uninstall_model,
            transcribe_addons_status,
            transcribe_download_vad,
            transcribe_uninstall_vad,
            transcribe_path_exists,
            transcribe_start,
            transcribe_cancel,
            transcribe_check_file_exists,
            sync_auto_sync,
            sync_cancel_auto_sync,
            refine_load_file,
            refine_save_file,
            refine_card_llm_tiered,
            refine_cards_llm_tiered,
            refine_cancel,
            ankiconnect_ping,
            ankiconnect_deck_names,
            ankiconnect_import_package,
            config_load_all,
            config_set,
            config_remove,
            config_clear,
            config_replace_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

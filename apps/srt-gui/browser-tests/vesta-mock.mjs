import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const subtitle = "/fixtures/Episode01.ja.srt";
const video = "/fixtures/Episode01.mp4";
const output = "/exports";
const text = "Synthetic subtitle for the browser test";
const calls = [];
const preferences = {
  "vesta-first-run-setup-complete": "true",
  "vesta-automatic-update-checks": "false",
  "vesta-export-format": "apkg",
  "vesta-default-llm-provider": "google",
  "vesta-default-llm-model": "gemini-fixture",
  "srt-tools-api-keys": JSON.stringify([
    { id: "fixture", name: "Fixture", apiType: "google", apiKey: "fixture-key", capabilities: ["llm"] },
    { id: "speech", name: "Speech fixture", apiType: "groq", apiKey: "fixture-key", capabilities: ["stt"] },
  ]),
  "vesta-transcribe-tiers": JSON.stringify([
    { id: "local", entries: [{ id: "local-model", provider: "local", model: "base" }] },
    { id: "cloud", entries: [{ id: "cloud-model", provider: "groq", model: "whisper-large-v3", apiKeyId: "speech" }] },
  ]),
  "vesta-translate-tiers": JSON.stringify([{ id: "fixture", entries: [{ provider: "google", model: "gemini-fixture", apiKeyId: "fixture" }] }]),
};
const params = new URLSearchParams(location.search);
const language = params.get("language") ?? "en";
const forceSetup = params.get("setup") === "true";
let finishExport;
let previewError;
let nextFile;
let mediaError;
let anchor;
let undoneAnchor;
let syncMedia = null;
let finishTranscription;
let finishTranslation;
let subtitleTracks = [
  {
    index: 2,
    codec: "hdmv_pgs_subtitle",
    language: "eng",
    title: "English PGS",
    text_based: false,
  },
];
const savedSubtitleOutputs = new Set();
const downloadedOcrModels = new Set();
const OCR_CATALOG = [
  { id: "paddleocr-v4-mobile-multi", name: "PaddleOCR v4 Mobile (Multilingual)", size: "~11 MB" },
  { id: "paddleocr-v4-mobile-cjk", name: "PaddleOCR v4 Mobile (CJK)", size: "~11 MB" },
  { id: "paddleocr-v4-mobile-latin", name: "PaddleOCR v4 Mobile (Latin)", size: "~8 MB" },
];
let supportLogRecording = false;
let supportLogPath = null;
let installedPlugins = [];
const downloadedFonts = new Set(["noto-sans-jp"]);
let demoPluginCheckRan = false;
let refineCards = [
  { id: "card-1", expression: "こんにちは", meaning: "Hello", notes: "" },
  { id: "card-2", expression: "ありがとう", meaning: "Thank you", notes: "Existing note" },
];
const DEMO_PLUGIN_MANIFEST = {
  id: "com.vesta.demo-plugin",
  name: "Demo Study Plugin",
  version: "1.0.0",
  apiVersion: 1,
  description: "Fixture plugin for Playwright E2E coverage",
  author: "Vesta Fixture",
  entry: { command: ["python3", "main.py"] },
  permissions: ["downloads", "network"],
  tab: { title: "Demo Study" },
  settings: [{ id: "apiEndpoint", label: "Endpoint", type: "text", defaultValue: "https://example.invalid" }],
};
function demoPluginView() {
  return {
    root: {
      type: "column",
      gap: 12,
      children: [
        { type: "heading", text: "Demo Study Workspace", level: 2 },
        {
          type: "text",
          id: "plugin-status-text",
          text: demoPluginCheckRan ? "Plugin action completed: 2 items verified." : "Ready to inspect workspace.",
        },
        {
          type: "button",
          id: "run_check",
          label: "Run Check",
          variant: "primary",
        },
      ],
    },
  };
}

const syncStatus = () => ({
  is_loaded: true, srt_path: subtitle, video_path: syncMedia,
  total_subtitles: 1, anchor_count: Number(Boolean(anchor)),
  checked_count: Number(Boolean(anchor)), completion_percentage: anchor ? 100 : 0,
  average_offset_ms: anchor?.offset_ms ?? 0, suggested_next_id: anchor ? null : 1,
  can_undo: Boolean(anchor), can_redo: Boolean(undoneAnchor),
});
const syncLine = () => ({
  id: 1, start_ms: 1000, end_ms: 2000, text,
  synced_start_ms: anchor?.corrected_time_ms ?? 1000,
  synced_end_ms: (anchor?.corrected_time_ms ?? 1000) + 1000,
  offset_ms: anchor?.offset_ms ?? 0, is_anchor: Boolean(anchor),
});

mockWindows("main");
mockIPC((command, args) => {
  calls.push({ command, args });
  switch (command) {
    case "config_load_all":
      return {
        entries: {
          ...preferences,
          "srt-tools-ui-language": language,
          ...(forceSetup ? { "vesta-first-run-force": "true" } : {}),
        },
        recovered_backup_path: null,
      };
    case "config_set":
      preferences[args.key] = args.value;
      return null;
    case "config_remove":
      delete preferences[args.key];
      return null;
    case "config_replace_all":
      for (const k of Object.keys(preferences)) delete preferences[k];
      Object.assign(preferences, args.values || {});
      return null;
    case "config_reset_installation":
      for (const k of Object.keys(preferences)) delete preferences[k];
      window.__onMockResetInstallation?.();
      return null;
    case "get_app_info":
      return { version: "0.34.0", name: "Vesta", license: "GPL-3.0-only" };
    case "get_mcp_cli_info":
      return {
        command: "/usr/bin/vesta-cli",
        bundled: true,
        default_read_root: "/fixtures",
        default_output_root: "/exports",
      };
    case "get_update_installation":
      return { channel: "tarball", os: "linux", arch: "x86_64" };
    case "install_release_update":
      return null;
    case "http_fetch":
      return {
        status: 200,
        url: args.req?.url || "https://example.invalid",
        headers: [["content-type", "application/json"]],
        body: JSON.stringify({ tag_name: "v0.34.0", data: [], models: [] }),
      };
    case "read_subtitle_file":
      return `1\n00:00:01,000 --> 00:00:02,000\n${args.path.endsWith("reference.srt") ? "Reference subtitle" : text}\n`;
    case "parse_subtitle_file":
      return [
        {
          id: 1,
          start: "00:00:01,000",
          end: "00:00:02,000",
          text: args.path.endsWith("reference.srt") ? "Reference subtitle" : text,
        },
      ];
    case "save_subtitle_file":
      return null;
    case "sync_load_srt":
    case "sync_get_status":
      return syncStatus();
    case "sync_get_subtitles_range":
      return [syncLine()];
    case "sync_get_subtitle":
      return syncLine();
    case "sync_get_anchors":
      return anchor ? [anchor] : [];
    case "sync_prepare_media_for_playback":
      if (mediaError) throw new Error(mediaError);
      return args.path;
    case "get_media_server_info":
      return [12345, "fixture"];
    case "sync_set_video":
      syncMedia = args.path;
      return syncStatus();
    case "sync_add_anchor":
      anchor = {
        subtitle_id: args.subtitleId, original_time_ms: 1000,
        corrected_time_ms: args.correctedTimeMs, offset_ms: args.correctedTimeMs - 1000, is_manual: true,
      };
      undoneAnchor = null;
      return syncStatus();
    case "sync_remove_anchor":
      anchor = null;
      return syncStatus();
    case "sync_undo":
      undoneAnchor = anchor;
      anchor = null;
      return syncStatus();
    case "sync_redo":
      anchor = undoneAnchor;
      undoneAnchor = null;
      return syncStatus();
    case "sync_auto_sync":
      anchor = {
        subtitle_id: 1,
        original_time_ms: 1000,
        corrected_time_ms: 1500,
        offset_ms: 500,
        is_manual: false,
      };
      undoneAnchor = null;
      return {
        success: true,
        cancelled: false,
        anchors_created: 1,
        segments_analyzed: 4,
        message: "Created 1 anchor",
      };
    case "sync_cancel_auto_sync":
      return null;
    case "sync_load_session":
      return syncStatus();
    case "sync_reset":
      anchor = null;
      undoneAnchor = null;
      syncMedia = null;
      return {
        is_loaded: false,
        srt_path: null,
        video_path: null,
        total_subtitles: 0,
        anchor_count: 0,
        checked_count: 0,
        completion_percentage: 0,
        average_offset_ms: 0,
        suggested_next_id: null,
        can_undo: false,
        can_redo: false,
      };
    case "sync_save_file":
      return args.outputPath || "/exports/synced.srt";
    case "sync_save_session":
      return args.sessionPath || "/exports/sync-session.vestasync";
    case "sync_suggest_companion_subtitle_for_srt":
    case "sync_suggest_media_for_srt":
      return null;
    case "sync_suggest_subtitles_for_media":
      return { target: null, native: null };
    case "load_srt_for_translate":
      return { path: args.path, subtitle_count: 1, coverage_end_ms: 2000, automatic_context: "", preview_subtitles: [{ id: 1, text }], first_subtitle: text, last_subtitle: text };
    case "suggest_translation_context":
      return "Conversational drama dialogue context.";
    case "get_latest_translated_subtitles":
      return [];
    case "start_translation":
      return new Promise((resolve, reject) => { finishTranslation = { resolve, reject }; });
    case "cancel_translation":
      finishTranslation?.resolve({ success: false, message: "Cancelled", output_path: null, translated_count: 0 });
      return true;
    case "transcribe_start":
      return new Promise((resolve, reject) => { finishTranscription = { resolve, reject }; });
    case "transcribe_cancel":
      finishTranscription?.reject(new Error("Cancelled"));
      return null;
    case "transcribe_check_backends":
      return {
        ffmpeg: true,
        whisper_cpp: true,
        python_whisper: false,
        whisper_binary: "whisper-rs (native)",
        any_whisper: true,
      };
    case "transcribe_check_file_exists":
    case "transcribe_path_exists":
      return true;
    case "transcribe_list_models":
      return [{ id: "base", name: "Base", downloaded: true }, { id: "small", name: "Small", downloaded: true }];
    case "transcribe_download_model":
    case "transcribe_uninstall_model":
    case "transcribe_download_vad":
    case "transcribe_uninstall_vad":
      return true;
    case "plugin:path|resolve_directory":
      return "/downloads";
    case "plugin:path|join":
      return (args.paths ?? []).filter(Boolean).join("/").replace(/\/+/g, "/");
    case "embedded_subtitle_tracks":
      return subtitleTracks;
    case "existing_subtitle_outputs":
      return (args.names ?? []).filter((name) => savedSubtitleOutputs.has(name));
    case "preview_embedded_subtitle":
      return "1\n00:00:01,000 --> 00:00:02,000\nEmbedded text track preview\n";
    case "extract_embedded_subtitle": {
      const filename = String(args.outputPath ?? "").split(/[\\/]/).pop();
      if (filename) savedSubtitleOutputs.add(filename);
      return null;
    }
    case "open_output_path":
      return null;
    case "preview_ocr_embedded_subtitle":
      return "1\n00:00:01,000 --> 00:00:02,000\nSynthetic OCR preview\n";
    case "ocr_embedded_subtitle": {
      const filename = String(args.outputPath ?? "").split(/[\\/]/).pop();
      if (filename) savedSubtitleOutputs.add(filename);
      return emit("ocr-progress", {
        stage: "done",
        message: "OCR completed",
        percentage: 100,
        current: 1,
        total: 1,
      }).then(() => null);
    }
    case "ocr_list_models":
      return OCR_CATALOG.map((model) => ({ ...model, downloaded: downloadedOcrModels.has(model.id) }));
    case "ocr_download_model": {
      downloadedOcrModels.add(args.modelId);
      return emit("ocr-progress", {
        stage: "download",
        message: `Downloaded ${args.modelId}`,
        percentage: 100,
        current: 1,
        total: 1,
      }).then(() => true);
    }
    case "ocr_uninstall_model":
      return downloadedOcrModels.delete(args.modelId);
    case "ocr_video_preview_frame":
      return {
        data_url: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='640' height='360'%3E%3Crect width='640' height='360' fill='%23111827'/%3E%3C/svg%3E",
        width: 1920,
        height: 1080,
        duration_ms: 120000,
        time_ms: args.timeMs ?? 10000,
      };
    case "preview_ocr_hardcoded_subtitle":
      return emit("ocr-progress", {
        stage: "ocr",
        message: "Previewing hardcoded ROI",
        percentage: 100,
        current: 2,
        total: 2,
      }).then(() => "1\n00:00:01,000 --> 00:00:03,500\nHardcoded subtitle line 1\n\n2\n00:00:04,000 --> 00:00:06,500\nHardcoded subtitle line 2\n");
    case "ocr_hardcoded_subtitle": {
      const filename = String(args.outputPath ?? "").split(/[\\/]/).pop();
      if (filename) savedSubtitleOutputs.add(filename);
      return emit("ocr-progress", {
        stage: "done",
        message: "Hardcoded OCR completed",
        percentage: 100,
        current: 2,
        total: 2,
      }).then(() => null);
    }
    case "ocr_cancel":
      return true;
    case "get_system_diagnostics":
      return {
        os: "linux",
        arch: "x86_64",
        compute_backend: "cpu",
        gpu_compiled: false,
        gpu_device: null,
        ffmpeg_available: true,
        video_encoder: "libx264",
        hardware_video_encoder: false,
        gstreamer_available: true,
        gstreamer_h264: true,
        gstreamer_h265: true,
      };
    case "support_log_status":
      return { path: supportLogPath, recording: supportLogRecording };
    case "support_log_start":
      supportLogRecording = true;
      supportLogPath = supportLogPath || "/exports/vesta-support.jsonl";
      return supportLogPath;
    case "support_log_stop":
      supportLogRecording = false;
      return null;
    case "support_log_append":
      return null;
    case "support_log_export":
      return args.destination || "/exports/vesta-support.jsonl";
    case "setup_media_status":
      return { ready: true, download_bytes: null };
    case "transcribe_addons_status":
      return { vad_models: [{ id: "v6.2.0", size: "~0.9 MB", downloaded: true }, { id: "webrtc", size: "", downloaded: true }], gpu_supported: false };
    case "plugin:dialog|open":
      if (args.options.directory) return output;
      if (nextFile) {
        const selected = nextFile;
        nextFile = null;
        return selected;
      }
      return [subtitle, video];
    case "plugin:dialog|save":
      if (nextFile) {
        const selected = nextFile;
        nextFile = null;
        return selected;
      }
      return "/exports/output.srt";
    case "flashcard_check_deps":
    case "flashcard_check_dir_exists":
      return true;
    case "plugin_list":
      return installedPlugins;
    case "plugin_inspect":
      return {
        manifest: DEMO_PLUGIN_MANIFEST,
        source: { originalUrl: args.url, gitCloneUrl: args.url, gitRef: args.gitRef || "main" },
        readme: "# Demo Study Plugin",
      };
    case "plugin_install": {
      const installed = {
        id: DEMO_PLUGIN_MANIFEST.id,
        name: DEMO_PLUGIN_MANIFEST.name,
        version: DEMO_PLUGIN_MANIFEST.version,
        description: DEMO_PLUGIN_MANIFEST.description,
        author: DEMO_PLUGIN_MANIFEST.author,
        enabled: true,
        sourceUrl: args.url,
        sourceRef: args.gitRef || "main",
        installedAt: "2026-11-18T12:00:00Z",
        manifest: DEMO_PLUGIN_MANIFEST,
        settings: { apiEndpoint: "https://example.invalid" },
      };
      installedPlugins = [...installedPlugins.filter((p) => p.id !== installed.id), installed];
      demoPluginCheckRan = false;
      return installed;
    }
    case "plugin_enable":
      installedPlugins = installedPlugins.map((p) => (p.id === args.id ? { ...p, enabled: true } : p));
      return null;
    case "plugin_disable":
      installedPlugins = installedPlugins.map((p) => (p.id === args.id ? { ...p, enabled: false } : p));
      return null;
    case "plugin_uninstall":
      installedPlugins = installedPlugins.filter((p) => p.id !== args.id);
      return null;
    case "plugin_update":
      return installedPlugins.find((p) => p.id === args.id) ?? null;
    case "plugin_get_settings":
      return installedPlugins.find((p) => p.id === args.id)?.settings ?? {};
    case "plugin_save_settings":
      installedPlugins = installedPlugins.map((p) => (p.id === args.id ? { ...p, settings: { ...args.settings } } : p));
      return null;
    case "plugin_render":
      return demoPluginView();
    case "plugin_dispatch_event":
      if (args.event?.widgetId === "run_check") demoPluginCheckRan = true;
      return demoPluginView();
    case "refine_load_file":
      return refineCards.map((c) => ({ ...c }));
    case "refine_card_llm_tiered":
      return "Grammar note from AI";
    case "refine_cards_llm_tiered": {
      const cards = Array.isArray(args.cards) ? args.cards : refineCards;
      const runId = args.runId ?? args.run_id ?? 1;
      const total = cards.length;
      return Promise.all(
        cards.map((c, idx) =>
          emit("refine-progress", {
            runId,
            event: {
              type: "cardDone",
              id: c.id,
              notes: "Grammar note from AI",
              done: idx + 1,
              total,
            },
          }),
        ),
      ).then(() => ({
        done: total,
        failed: 0,
        remaining: 0,
        poolExhausted: false,
        cancelled: false,
      }));
    }
    case "refine_cancel":
      return true;
    case "refine_save_file":
      for (const upd of args.updates || []) {
        const card = refineCards.find((c) => c.id === upd.id);
        if (card) card.notes = upd.notes;
      }
      return true;
    case "ankiconnect_ping":
      return 6;
    case "ankiconnect_import_package":
      return true;
    case "ankiconnect_deck_names":
      return [];
    case "flashcard_list_fonts":
      return [{
        id: "noto-sans-jp",
        name: "Noto Sans JP",
        language_name: "Japanese",
        target_languages: ["ja", "jpn"],
        filename: "_NotoSansJP.ttf",
        approx_size: "~11 MB",
        downloaded: downloadedFonts.has("noto-sans-jp"),
      }];
    case "flashcard_download_font":
      if (args.fontId) downloadedFonts.add(args.fontId);
      return true;
    case "flashcard_delete_font":
      return downloadedFonts.delete(args.fontId);
    case "flashcard_download_ffmpeg":
      return true;
    case "flashcard_get_cpu_count":
      return 4;
    case "flashcard_get_total_memory_mb":
      return 16384;
    case "flashcard_list_audio_tracks":
      return [{ index: 0, language: "ja", title: "Japanese", codec: "aac" }];
    case "flashcard_media_duration":
      return 12;
    case "flashcard_parse_subtitles":
      return [{
        id: 1,
        start_ms: 500,
        end_ms: 1500,
        text,
        actor: null,
        style: null,
        active: true,
      }];
    case "flashcard_preview":
      if (previewError) throw new Error(previewError);
      return [{
        index: 0,
        start_ms: 500,
        end_ms: 1500,
        duration_ms: 1000,
        subs1_text: text,
        subs2_text: args.config?.native_subs_path ? "Reference subtitle" : null,
        actor: null,
        active: true,
        leading_context: [],
        trailing_context: [],
      }];
    case "flashcard_preview_audio":
      return "/exports/preview-audio.mp3";
    case "flashcard_preview_snapshot":
      return "/exports/preview-snapshot.jpg";
    case "flashcard_merge_apkg":
      return args.outputPath || "/exports/merged.apkg";
    case "flashcard_export_dueling":
      return {
        output_path: args.options?.output_path || "/exports/Episode01.dueling.ass",
        format: args.options?.format || "ass",
        line_count: 1,
        matched_count: args.config?.native_subs_path ? 1 : 0,
      };
    case "save_temp_subtitles":
      return args.useNative ? "/fixtures/preview-native.srt" : "/fixtures/preview-target.srt";
    case "flashcard_generate":
      return new Promise((resolve) => { finishExport = resolve; });
    case "flashcard_cancel":
      finishExport?.({
        success: false,
        message: "Cancelled by user",
        cards_generated: 0,
        audio_clips: 0,
        snapshots: 0,
        video_clips: 0,
        tsv_path: null,
        apkg_path: null,
        output_size_bytes: 0,
      });
      return true;
    default:
      return null;
  }
}, { shouldMockEvents: true });

window.browserFixture = {
  calls,
  selectFile: (path) => { nextFile = path; },
  setSubtitleTracks: (tracks) => { subtitleTracks = tracks; },
  failMedia: (message) => { mediaError = message; },
  failTranscription: (message) => finishTranscription.reject(new Error(message)),
  completeTranscription: () => finishTranscription.resolve({
    success: true, message: "Transcription completed: 1 segment", subtitle_count: 1, output_path: "/exports/output.srt",
  }),
  failTranslation: (message) => finishTranslation.reject(new Error(message)),
  completeTranslation: () => finishTranslation.resolve({
    success: true, message: "Translation completed: 1 subtitle", output_path: "/exports/translated.srt", translated_count: 1,
  }),
  failPreview: (message) => { previewError = message; },
  progress: () => emit("flashcard-progress", {
    percentage: 40,
    stage: "audio",
    message: "flashcards.progress.parsing",
    current: 4,
    total: 10,
    params: {},
  }),
  complete: () => finishExport({
    success: true,
    message: "Generated 1 cards (1 audio, 1 snapshots, 0 video clips)",
    cards_generated: 1,
    audio_clips: 1,
    snapshots: 1,
    video_clips: 0,
    tsv_path: null,
    apkg_path: output + "/Episode01.apkg",
    output_size_bytes: 4096,
  }),
};

await import("../src/main.ts");

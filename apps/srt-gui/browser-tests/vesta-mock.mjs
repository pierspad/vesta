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
const language = new URLSearchParams(location.search).get("language") ?? "en";
let finishExport;
let previewError;
let nextFile;
let mediaError;
let anchor;
let undoneAnchor;
let syncMedia = null;
let finishTranscription;
let finishTranslation;

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
      return { ...preferences, "srt-tools-ui-language": language };
    case "config_set":
      preferences[args.key] = args.value;
      return null;
    case "read_subtitle_file":
      return `1\n00:00:01,000 --> 00:00:02,000\n${args.path.endsWith("reference.srt") ? "Reference subtitle" : text}\n`;
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
    case "load_srt_for_translate":
      return { path: args.path, subtitle_count: 1, coverage_end_ms: 2000, automatic_context: "", preview_subtitles: [{ id: 1, text }], first_subtitle: text, last_subtitle: text };
    case "get_latest_translated_subtitles":
      return [];
    case "start_translation":
      return new Promise((resolve, reject) => { finishTranslation = { resolve, reject }; });
    case "cancel_translation":
      finishTranslation.resolve({ success: false, message: "Cancelled", output_path: null, translated_count: 0 });
      return null;
    case "transcribe_start":
      return new Promise((resolve, reject) => { finishTranscription = { resolve, reject }; });
    case "transcribe_cancel":
      finishTranscription.reject(new Error("Cancelled"));
      return null;
    case "transcribe_check_file_exists":
    case "transcribe_path_exists":
      return true;
    case "transcribe_list_models":
      return [{ id: "base", name: "Base", downloaded: true }, { id: "small", name: "Small", downloaded: true }];
    case "plugin:path|resolve_directory":
      return "/downloads";
    case "transcribe_addons_status":
      return { vad_models: [] };
    case "plugin:dialog|open":
      if (args.options.directory) return output;
      if (nextFile) {
        const selected = nextFile;
        nextFile = null;
        return selected;
      }
      return [subtitle, video];
    case "plugin:dialog|save":
      return "/exports/output.srt";
    case "flashcard_check_deps":
    case "flashcard_check_dir_exists":
      return true;
    case "plugin_list":
    case "ankiconnect_deck_names":
    case "flashcard_list_fonts":
      return [];
    case "flashcard_get_cpu_count":
      return 4;
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
        subs1_text: text,
        subs2_text: null,
        active: true,
        leading_context: [],
        trailing_context: [],
      }];
    case "save_temp_subtitles":
      return "/fixtures/preview-target.srt";
    case "flashcard_generate":
      return new Promise((resolve) => { finishExport = resolve; });
    case "flashcard_cancel":
      finishExport({ success: false, message: "Cancelled by user" });
      return true;
    default:
      return null;
  }
}, { shouldMockEvents: true });

window.browserFixture = {
  calls,
  selectFile: (path) => { nextFile = path; },
  failMedia: (message) => { mediaError = message; },
  failTranscription: (message) => finishTranscription.reject(new Error(message)),
  completeTranscription: () => finishTranscription.resolve({
    success: true, message: "Transcription completed: 1 segment", subtitle_count: 1, output_path: "/exports/output.srt",
  }),
  failTranslation: (message) => finishTranslation.reject(new Error(message)),
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
    cards_generated: 1,
    audio_clips: 1,
    snapshots: 1,
    video_clips: 0,
    tsv_path: null,
    apkg_path: output + "/Episode01.apkg",
  }),
};

await import("../src/main.ts");

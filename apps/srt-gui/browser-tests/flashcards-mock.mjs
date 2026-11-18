import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

const subtitle = "/fixtures/Episode01.ja.srt";
const video = "/fixtures/Episode01.mp4";
const output = "/exports";
const text = "Synthetic subtitle for the browser test";
const calls = [];
let finishExport;
let previewError;

mockWindows("main");
mockIPC((command, args) => {
  calls.push({ command, args });
  switch (command) {
    case "config_load_all":
      return {
        "vesta-first-run-setup-complete": "true",
        "vesta-automatic-update-checks": "false",
        "vesta-export-format": "apkg",
        "srt-tools-ui-language": "en",
      };
    case "plugin:path|resolve_directory":
      return "/downloads";
    case "transcribe_addons_status":
      return { vad_models: [] };
    case "plugin:dialog|open":
      return args.options.directory ? output : [subtitle, video];
    case "flashcard_check_deps":
    case "flashcard_check_dir_exists":
      return true;
    case "transcribe_list_models":
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

window.flashcardsFixture = {
  calls,
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

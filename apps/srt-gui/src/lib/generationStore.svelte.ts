import { ankiStore } from "./ankiStore.svelte";
import type { LogEntry } from "./LogPanel.svelte";
import * as vestaConfig from "./vestaConfig";

export const EXPORT_FORMAT_KEY = "vesta-export-format";
export const SERIES_OUTPUT_MODE_KEY = "vesta-series-output-mode";

export interface GenerationResult {
  success: boolean;
  message?: string | null;
  cardsGenerated: number;
  audioClips: number;
  snapshots: number;
  videoClips: number;
  tsvPath: string | null;
  apkgPath: string | null;
}

function loadInitialExportFormat(): "tsv" | "apkg" | "anki" {
  try {
    const saved = vestaConfig.getItem(EXPORT_FORMAT_KEY);
    if (saved === "tsv" || saved === "anki" || saved === "apkg") return saved;
  } catch {}
  return "apkg";
}

function loadInitialSeriesOutputMode(): "single" | "separate" {
  try {
    return vestaConfig.getItem(SERIES_OUTPUT_MODE_KEY) === "single" ? "single" : "separate";
  } catch {
    return "separate";
  }
}

/**
 * Generation run-state (progress/log/result of the current flashcard-export
 * run) plus the output settings that feed it (export format, series output
 * mode, deck name, CPU cores). Deliberately does NOT own domain data
 * (episodes, subtitle paths, note-type, media/card-filter settings) — those
 * stay in FlashcardsTab.svelte's buildConfig()/startGeneration()/
 * startSeriesGeneration(), which read this store's fields alongside their
 * own local state (see [[vesta-flashcards-refactor]] memory for why: a
 * $derived declared *inside* this store class cannot see FlashcardsTab's
 * local $state, so validation deriveds like canRunFlashcards/
 * generationRequirements stay in the component, not here).
 *
 * Also deliberately has NO $effect — the three exportFormat-sync effects
 * (persist on change, resync when the tab becomes active, resync on
 * ankiStore.status changes) and the seriesOutputMode-persist effect stay in
 * FlashcardsTab.svelte because they react to `active` (a component prop)
 * and to the component's mount lifecycle, matching the store convention
 * used by every other store in this codebase.
 */
class GenerationStore {
  isProcessing = $state(false);
  progress = $state(0);
  progressMessage = $state("");
  progressStage = $state("");

  logs = $state<LogEntry[]>([]);
  error = $state<string | null>(null);
  result = $state<GenerationResult | null>(null);

  deckName = $state("");
  deckNameAuto = $state(true);

  exportFormat = $state<"tsv" | "apkg" | "anki">(loadInitialExportFormat());
  seriesOutputMode = $state<"single" | "separate">(loadInitialSeriesOutputMode());

  cpuCores = $state(2);
  systemCpuCount = $state(4);

  effectiveExportFormat = $derived(this.exportFormat === "anki" ? "apkg" : this.exportFormat);
  /** Alias kept for parity with effectiveExportFormat; cpuCores has no other
   * derived transform today, but buildConfig() reads this name. */
  effectiveCpuCores = $derived(this.cpuCores);

  private logIdCounter = 0;
  private lastProgressKey: string | null = null;

  addLog(message: string, type: LogEntry["type"] = "info", details?: string, progressKey?: string) {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    // For sequential progress messages with the same key, update in-place
    if (type === "progress" && progressKey && progressKey === this.lastProgressKey && this.logs.length > 0) {
      const last = this.logs[this.logs.length - 1];
      if (last.type === "progress") {
        const updated = { ...last, timestamp, message };
        this.logs = [...this.logs.slice(0, -1), updated];
        return;
      }
    }

    if (type === "progress" && progressKey) {
      this.lastProgressKey = progressKey;
    } else if (type !== "progress") {
      this.lastProgressKey = null;
    }

    this.logs = [...this.logs, { id: ++this.logIdCounter, timestamp, message, type, details }];
  }

  clearLogs() {
    this.logs = [];
    this.lastProgressKey = null;
  }

  /** Resets only the run-state (result/error/progress/logs) — NOT deckName
   * and NOT any domain data. FlashcardsTab.svelte's resetGeneration() also
   * clears episodes/subtitle paths/deckName and calls this for its part. */
  resetRun() {
    this.result = null;
    this.error = null;
    this.progress = 0;
    this.progressMessage = "";
    this.progressStage = "";
    this.logs = [];
    this.logIdCounter = 0;
    this.lastProgressKey = null;
  }

  /** Stops the visual "in progress" state after a cancel; the caller is
   * still responsible for invoking the backend cancel command and logging
   * a translated message (this store has no access to t()). */
  cancelRun() {
    this.isProcessing = false;
    this.progress = 0;
    this.progressMessage = "";
  }

  cycleExportFormat() {
    if (this.exportFormat === "apkg") {
      this.exportFormat = "tsv";
    } else if (this.exportFormat === "tsv") {
      this.exportFormat = ankiStore.status === "online" ? "anki" : "apkg";
    } else {
      this.exportFormat = "apkg";
    }
  }
}

export const generationStore = new GenerationStore();

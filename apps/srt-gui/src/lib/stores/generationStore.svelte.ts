import { ankiStore } from "$lib/stores/ankiStore.svelte";
import type { LogEntry } from "$lib/panels/LogPanel.svelte";
import * as vestaConfig from "$lib/config/vestaConfig";

export const EXPORT_FORMAT_KEY = "vesta-export-format";
export const EXPORT_FALLBACK_KEY = "vesta-export-fallback";
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

function loadInitialExportFallback(): "tsv" | "apkg" {
  try {
    const saved = vestaConfig.getItem(EXPORT_FALLBACK_KEY);
    if (saved === "tsv" || saved === "apkg") return saved;
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
 * mode, deck name, CPU cores).
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
  fallbackFormat = $state<"tsv" | "apkg">(loadInitialExportFallback());
  seriesOutputMode = $state<"single" | "separate">(loadInitialSeriesOutputMode());

  cpuCores = $state(2);
  systemCpuCount = $state(4);

  effectiveExportFormat = $derived<"tsv" | "apkg" | "anki">(
    this.exportFormat === "anki"
      ? (ankiStore.status === "online" ? "anki" : this.fallbackFormat)
      : this.exportFormat
  );
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

  cancelRun() {
    this.isProcessing = false;
    this.progress = 0;
    this.progressMessage = "";
  }

  setExportFormat(format: "tsv" | "apkg" | "anki") {
    this.exportFormat = format;
    try { vestaConfig.setItem(EXPORT_FORMAT_KEY, format); } catch {}
  }

  setFallbackFormat(fallback: "tsv" | "apkg") {
    this.fallbackFormat = fallback;
    try { vestaConfig.setItem(EXPORT_FALLBACK_KEY, fallback); } catch {}
  }

  cycleExportFormat() {
    if (this.exportFormat === "apkg") {
      this.setExportFormat("tsv");
    } else if (this.exportFormat === "tsv") {
      this.setExportFormat("anki");
    } else {
      this.setExportFormat("apkg");
    }
  }
}

export const generationStore = new GenerationStore();

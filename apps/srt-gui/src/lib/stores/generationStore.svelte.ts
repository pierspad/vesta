import type { LogEntry } from "$lib/panels/LogPanel.svelte";
import * as vestaConfig from "$lib/config/vestaConfig";
import {
  exportFormatStore,
  type ExportFallbackFormat,
  type ExportFormat,
} from "$lib/stores/exportFormatStore.svelte";

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

  seriesOutputMode = $state<"single" | "separate">(loadInitialSeriesOutputMode());

  cpuCores = $state(2);
  systemCpuCount = $state(4);

  // Export format lives in exportFormatStore (single source of truth, shared
  // with Settings -> Overview); these accessors only delegate.
  get exportFormat(): ExportFormat {
    return exportFormatStore.exportFormat;
  }
  set exportFormat(format: ExportFormat) {
    exportFormatStore.setExportFormat(format);
  }
  get fallbackFormat(): ExportFallbackFormat {
    return exportFormatStore.fallbackFormat;
  }
  set fallbackFormat(fallback: ExportFallbackFormat) {
    exportFormatStore.setFallbackFormat(fallback);
  }
  get effectiveExportFormat(): ExportFormat {
    return exportFormatStore.effectiveExportFormat;
  }
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

  setExportFormat(format: ExportFormat) {
    exportFormatStore.setExportFormat(format);
  }

  setFallbackFormat(fallback: ExportFallbackFormat) {
    exportFormatStore.setFallbackFormat(fallback);
  }

  cycleExportFormat() {
    exportFormatStore.cycleExportFormat();
  }
}

export const generationStore = new GenerationStore();

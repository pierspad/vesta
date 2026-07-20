import { ankiStore } from "./ankiStore.svelte";
import * as vestaConfig from "./vestaConfig";

export const EXPORT_FORMAT_KEY = "vesta-export-format";
export const EXPORT_FALLBACK_KEY = "vesta-export-fallback";
export type ExportFormat = "apkg" | "tsv" | "anki";
export type ExportFallbackFormat = "apkg" | "tsv";

function loadInitial(): ExportFormat {
  try {
    const saved = vestaConfig.getItem(EXPORT_FORMAT_KEY);
    if (saved === "tsv" || saved === "anki" || saved === "apkg") return saved;
    return "apkg";
  } catch {
    return "apkg";
  }
}

function loadInitialFallback(): ExportFallbackFormat {
  try {
    const saved = vestaConfig.getItem(EXPORT_FALLBACK_KEY);
    if (saved === "tsv" || saved === "apkg") return saved;
    return "apkg";
  } catch {
    return "apkg";
  }
}

/** Export-format toggle card in Settings -> Overview & FlashcardsTab.
 * State only -- all 3 options (apkg, tsv, anki) stay permanently visible.
 * If AnkiConnect is selected as preferred but Anki is offline, effectiveExportFormat
 * seamlessly resolves to fallbackFormat (apkg or tsv). */
class ExportFormatStore {
  exportFormat = $state<ExportFormat>(loadInitial());
  fallbackFormat = $state<ExportFallbackFormat>(loadInitialFallback());

  numOpts = 3;
  activeIdx = $derived(this.exportFormat === "apkg" ? 0 : this.exportFormat === "tsv" ? 1 : 2);

  effectiveExportFormat = $derived<ExportFormat>(
    this.exportFormat === "anki"
      ? (ankiStore.status === "online" ? "anki" : this.fallbackFormat)
      : this.exportFormat
  );

  setExportFormat(format: ExportFormat) {
    this.exportFormat = format;
    try {
      vestaConfig.setItem(EXPORT_FORMAT_KEY, format);
    } catch {}
  }

  setFallbackFormat(fallback: ExportFallbackFormat) {
    this.fallbackFormat = fallback;
    try {
      vestaConfig.setItem(EXPORT_FALLBACK_KEY, fallback);
    } catch {}
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

export const exportFormatStore = new ExportFormatStore();

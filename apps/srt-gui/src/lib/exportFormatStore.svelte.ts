import { ankiStore } from "./ankiStore.svelte";
import * as vestaConfig from "./vestaConfig";

export const EXPORT_FORMAT_KEY = "vesta-export-format";
export type ExportFormat = "apkg" | "tsv" | "anki";

function loadInitial(): ExportFormat {
  try {
    const saved = vestaConfig.getItem(EXPORT_FORMAT_KEY);
    if (saved === "tsv" || saved === "anki" || saved === "apkg") return saved;
    return "apkg";
  } catch {
    return "apkg";
  }
}

/** Export-format toggle card in Settings -> Overview (expert mode only).
 * State only -- the three `$effect`s that resync `exportFormat` from
 * localStorage on tab-activation/expert-mode-toggle, and persist it back to
 * localStorage on change, stay in SettingsTab.svelte: `$effect` requires a
 * component effect root and can't run inside a module-level store singleton,
 * see [[vesta-settings-refactor]]. `showAnki`/`numOpts` only need a plain
 * `$derived` (reading ankiStore's own reactive state), so those *do* live
 * here safely -- it's specifically the imperative self-referential
 * resync effects that can't. */
class ExportFormatStore {
  exportFormat = $state<ExportFormat>(loadInitial());

  showAnki = $derived(ankiStore.status === "online");
  numOpts = $derived(this.showAnki ? 3 : 2);
  activeIdx = $derived(this.exportFormat === "apkg" ? 0 : this.exportFormat === "tsv" ? 1 : 2);

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

export const exportFormatStore = new ExportFormatStore();

import { invoke } from "@tauri-apps/api/core";
import { smartMatchingStore } from "./smartMatchingStore.svelte";
import { snackbar } from "./snackbarStore.svelte";
import * as vestaConfig from "./vestaConfig";

/**
 * Global UI mode: Easy (default) vs Expert.
 *
 * Easy mode strips the UI down to the essential decisions (which media to
 * include, which audio track) and picks sane defaults for everything else:
 * automatic deck name, forced .apkg export, CPU cores = n-1, no advanced
 * audio/video parameters. Expert mode exposes the full surface.
 */
class UiModeStore {
  expertMode = $state(false);
  useOldReviseUi = $state(true);

  /** Convenience inverse — most call sites gate on "easy". */
  easyMode = $derived(!this.expertMode);

  constructor() {
    this.expertMode = vestaConfig.getItem("vesta-expert-mode") === "true";
    this.useOldReviseUi = true;
  }

  async toggleExpertMode() {
    const prevExpertMode = this.expertMode;
    const newExpertMode = !prevExpertMode;

    if (prevExpertMode && !newExpertMode) {
      // Disabling expert mode: Backup and set to easy defaults

      // 1. Export Format: backup and set to apkg
      const currentExport = vestaConfig.getItem("vesta-export-format") || "apkg";
      vestaConfig.setItem("vesta-expert-backup-export-format", currentExport);
      vestaConfig.setItem("vesta-export-format", "apkg");
    } else if (!prevExpertMode && newExpertMode) {
      // Enabling expert mode: Restore from backup

      // 1. Export Format
      const backupExport = vestaConfig.getItem("vesta-expert-backup-export-format");
      if (backupExport) {
        vestaConfig.setItem("vesta-export-format", backupExport);
      }
    }

    this.expertMode = newExpertMode;
    vestaConfig.setItem("vesta-expert-mode", String(this.expertMode));
  }

  toggleReviseUi() {
    this.useOldReviseUi = !this.useOldReviseUi;
    vestaConfig.setItem("vesta-use-old-revise-ui", String(this.useOldReviseUi));
    if (this.useOldReviseUi) {
      snackbar.show("UI Mode: Legacy Panels", "info");
    } else {
      snackbar.show("UI Mode: Modern Layout", "info");
    }
  }
}

export const uiMode = new UiModeStore();

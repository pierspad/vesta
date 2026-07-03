import { invoke } from "@tauri-apps/api/core";
import { smartMatchingStore } from "./smartMatchingStore.svelte";

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

  /** Convenience inverse — most call sites gate on "easy". */
  easyMode = $derived(!this.expertMode);

  constructor() {
    if (typeof localStorage !== "undefined") {
      this.expertMode = localStorage.getItem("vesta-expert-mode") === "true";
    }
  }

  async toggleExpertMode() {
    const prevExpertMode = this.expertMode;
    const newExpertMode = !prevExpertMode;

    if (typeof localStorage !== "undefined") {
      if (prevExpertMode && !newExpertMode) {
        // Disabling expert mode: Backup and set to easy defaults
        
        // 1. Export Format: backup and set to apkg
        const currentExport = localStorage.getItem("vesta-export-format") || "apkg";
        localStorage.setItem("vesta-expert-backup-export-format", currentExport);
        localStorage.setItem("vesta-export-format", "apkg");

        // 3. Smart Matching: backup and set to false
        localStorage.setItem("vesta-expert-backup-smart-matching-enabled", String(smartMatchingStore.enabled));
        smartMatchingStore.setEnabled(false);

      } else if (!prevExpertMode && newExpertMode) {
        // Enabling expert mode: Restore from backup
        
        // 1. Export Format
        const backupExport = localStorage.getItem("vesta-expert-backup-export-format");
        if (backupExport) {
          localStorage.setItem("vesta-export-format", backupExport);
        }

        // 3. Smart Matching
        const backupSmartMatching = localStorage.getItem("vesta-expert-backup-smart-matching-enabled");
        if (backupSmartMatching !== null) {
          smartMatchingStore.setEnabled(backupSmartMatching === "true");
        }
      }
    }

    this.expertMode = newExpertMode;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("vesta-expert-mode", String(this.expertMode));
    }
  }
}

export const uiMode = new UiModeStore();

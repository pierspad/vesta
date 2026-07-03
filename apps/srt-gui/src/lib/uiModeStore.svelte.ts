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

  toggleExpertMode() {
    this.expertMode = !this.expertMode;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("vesta-expert-mode", String(this.expertMode));
    }
  }
}

export const uiMode = new UiModeStore();

class AiStore {
  killSwitchActive = $state(false);
  isTranslating = $state(false);
  isTranscribing = $state(false);
  autoRefining = $state(false);
  isSingleRefining = $state(false);

  hasActiveAiProcess = $derived(
    this.isTranslating ||
    this.isTranscribing ||
    this.autoRefining ||
    this.isSingleRefining
  );

  constructor() {
    if (typeof localStorage !== "undefined") {
      this.killSwitchActive = localStorage.getItem("vesta-ai-kill-switch") === "true";
    }
  }

  toggleKillSwitch() {
    if (this.hasActiveAiProcess) {
      return;
    }
    this.killSwitchActive = !this.killSwitchActive;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("vesta-ai-kill-switch", String(this.killSwitchActive));
    }
  }
}

export const aiStore = new AiStore();

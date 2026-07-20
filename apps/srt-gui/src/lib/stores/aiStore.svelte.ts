import * as vestaConfig from "$lib/config/vestaConfig";

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
    this.killSwitchActive = vestaConfig.getItem("vesta-ai-kill-switch") === "true";
  }

  toggleKillSwitch() {
    if (this.hasActiveAiProcess) {
      return;
    }
    this.killSwitchActive = !this.killSwitchActive;
    vestaConfig.setItem("vesta-ai-kill-switch", String(this.killSwitchActive));
  }
}

export const aiStore = new AiStore();

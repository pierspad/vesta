import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
import * as vestaConfig from "$lib/config/vestaConfig";
import { invokeCommand } from "$lib/services/tauriClient";

class AiStore {
  killSwitchActive = $state(false);
  isTranslating = $state(false);
  isTranscribing = $state(false);
  autoRefining = $state(false);
  isSingleRefining = $state(false);
  isOcring = $state(false);

  hasActiveAiProcess = $derived(
    this.isTranslating ||
    this.isTranscribing ||
    this.autoRefining ||
    this.isSingleRefining ||
    this.isOcring ||
    (localWhisperOperation.busy && (localWhisperOperation.activeAction === "transcription" || localWhisperOperation.activeAction === "autosync"))
  );

  constructor() {
    this.killSwitchActive = vestaConfig.getItem("vesta-ai-kill-switch") === "true";
  }

  async cancelActiveAiProcesses(): Promise<void> {
    const commands = new Set<string>();
    if (this.isTranslating) commands.add("cancel_translation");
    if (
      this.isTranscribing ||
      (localWhisperOperation.busy && localWhisperOperation.activeAction === "transcription")
    ) {
      commands.add("transcribe_cancel");
    }
    if (this.autoRefining || this.isSingleRefining) commands.add("refine_cancel");
    if (this.isOcring) commands.add("ocr_cancel");
    if (localWhisperOperation.busy && localWhisperOperation.activeAction === "autosync") {
      commands.add("sync_cancel_auto_sync");
    }
    await Promise.all([...commands].map((cmd) => invokeCommand(cmd).catch(() => {})));
  }

  toggleKillSwitch() {
    this.killSwitchActive = !this.killSwitchActive;
    vestaConfig.setItem("vesta-ai-kill-switch", String(this.killSwitchActive));
    if (this.killSwitchActive && this.hasActiveAiProcess) {
      void this.cancelActiveAiProcesses();
    }
  }
}

export const aiStore = new AiStore();

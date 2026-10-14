import { localWhisperOperation, LatestRequest } from "$lib/utils/asyncControl.svelte";
import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { guardedOpen } from "$lib/utils/dialogGuard";
import {
  loadVadSelection,
  saveVadSelection,
  DEFAULT_VAD_MODEL_ID,
  type VadSelection,
} from "$lib/config/vadSelection";
import * as vestaConfig from "$lib/config/vestaConfig";
import {
  transcribeListModels,
  transcribeDownloadModel,
  transcribeCancel,
  transcribeUninstallModel,
  transcribeAddonsStatus,
  transcribePathExists,
  transcribeDownloadVad,
  transcribeUninstallVad,
  type WhisperModel,
  type VadModel,
} from "$lib/services/transcribe";

// Ri-esportati per compatibilità: i tipi vivono ora in services/transcribe.ts
// (è il modulo che conosce la forma dei dati restituiti da Rust).
export type { WhisperModel, VadModel };

export type WhisperContextMenu = {
  x: number;
  y: number;
  kind: "model" | "panel";
  modelId?: string;
  downloaded?: boolean;
} | null;

function showSnackbar(message: string, type: "error" | "success" = "success") {
  snackbar.show(message, type === "error" ? "error" : "success");
}

/** Local Whisper model downloads + Silero VAD add-ons, and the context menu
 * that manages them. Self-contained to SettingsTab.svelte's "whisper"
 * section EXCEPT: `whisperModels`/`downloadedWhisperCount` are also read by
 * the "overview" section's quick-setup checklist and by a cross-section
 * $effect that publishes global action-needed state (both stay in
 * SettingsTab.svelte, reading this store's fields directly -- moving the
 * *state* here doesn't require moving those *readers*). `highlightedModelId`
 * is genuinely shared with the llm section's refinement-prompt editor, so it
 * deliberately stayed OUT of this store (passed into the panel as a prop) --
 * see [[vesta-settings-refactor]]. */
export class WhisperModelsStore {
  private modelRequests = new LatestRequest();
  private addonRequests = new LatestRequest();
  whisperModels = $state<WhisperModel[]>([
    { id: "tiny", name: "Tiny", size: "~75MB", speed: "~32x", downloaded: false },
    { id: "base", name: "Base", size: "~142MB", speed: "~16x", downloaded: false },
    { id: "small", name: "Small", size: "~466MB", speed: "~6x", downloaded: false },
    { id: "medium", name: "Medium", size: "~1.5GB", speed: "~2x", downloaded: false },
    { id: "large", name: "Large", size: "~3.1GB", speed: "~1x", downloaded: false },
  ]);
  downloadedWhisperCount = $derived(this.whisperModels.filter((model) => model.downloaded).length);

  busy = $derived(localWhisperOperation.busy);
  isDownloading = $state(false);
  isCancellingDownload = $state(false);
  downloadingModelId = $state<string | null>(null);
  pendingDefaultModelId = $state<string | null>(null);
  progress = $state(0);
  progressMessage = $state("");
  progressStage = $state("");
  defaultWhisperModel = $state("base");

  contextMenu = $state<WhisperContextMenu>(null);

  vadModels = $state<VadModel[]>([]);
  vadSelection = $state<VadSelection>(loadVadSelection());
  downloadingVadId = $state<string | null>(null);
  vadCustomValid = $state(false);

  async refreshModels() {
    const request = this.modelRequests.begin();
    try {
      const models = await transcribeListModels();
      if (!request.isCurrent()) return;
      this.whisperModels = models;
      window.dispatchEvent(new CustomEvent("vesta-whisper-models-updated", { detail: { models } }));
    } catch (e) {
      console.error("Could not list models:", e);
    }
  }

  setDefaultWhisperModel(modelId: string, notify = true) {
    if (this.busy) return;
    this.defaultWhisperModel = modelId;
    vestaConfig.setItem("srt-default-whisper-model", modelId);
    if (notify) {
      showSnackbar(t("settings.whisper.defaultSet", { model: modelId }));
    }
    // Dispatch event so other tabs can pick up the change if needed
    window.dispatchEvent(new CustomEvent("whisper-model-updated", { detail: modelId }));
  }

  handleWhisperModelClick(model: { id: string; downloaded: boolean }) {
    if (this.busy) return;
    if (model.downloaded) {
      this.setDefaultWhisperModel(model.id);
      return;
    }
    void this.downloadModel(model.id, true);
  }

  async downloadModel(modelId: string, setAsDefaultAfterDownload = false) {
    const run = localWhisperOperation.begin("download");
    if (!run) return false;
    this.isDownloading = true;
    this.isCancellingDownload = false;
    this.downloadingModelId = modelId;
    this.pendingDefaultModelId = setAsDefaultAfterDownload ? modelId : null;
    try {
      await transcribeDownloadModel(modelId);
      await this.refreshModels();

      const downloaded = this.whisperModels.find((m) => m.id === modelId)?.downloaded;
      if (run.isCancelled()) return false;
      if (!downloaded) throw new Error(t("setup.resourceCheckFailed"));
      if (this.pendingDefaultModelId === modelId) {
        this.defaultWhisperModel = modelId;
        vestaConfig.setItem("srt-default-whisper-model", modelId);
        window.dispatchEvent(new CustomEvent("whisper-model-updated", { detail: modelId }));
        showSnackbar(t("settings.whisper.downloadAndSetSuccess", { model: modelId }));
      } else if (downloaded) {
        showSnackbar(t("settings.whisper.downloadSuccess", { model: modelId }));
      }
      return true;
    } catch (e) {
      const message = String(e).toLowerCase();
      if (message.includes("cancelled") || message.includes("canceled")) {
        snackbar.show(t("settings.modelDownloadCancelled", { model: modelId }), "info");
      } else {
        showSnackbar(t("settings.whisper.downloadFailed", { model: modelId, error: String(e) }), "error");
      }
      return false;
    } finally {
      run.finish();
      this.isDownloading = false;
      this.isCancellingDownload = false;
      this.downloadingModelId = null;
      this.pendingDefaultModelId = null;
      this.progress = 0;
      this.progressMessage = "";
      this.progressStage = "";
      window.dispatchEvent(new CustomEvent("vesta-whisper-download-progress", {
        detail: { modelId: null, progress: 0 }
      }));
    }
  }

  async cancelModelDownload() {
    if ((!this.isDownloading && !this.downloadingVadId) || this.isCancellingDownload) return;
    this.isCancellingDownload = true;
    try {
      await localWhisperOperation.cancel(transcribeCancel);
    } catch (e) {
      showSnackbar(t("settings.whisper.cancelFailed", { error: String(e) }), "error");
      this.isCancellingDownload = false;
    }
  }

  async uninstallModel(modelId: string) {
    const run = localWhisperOperation.begin("uninstall");
    if (!run) return;
    try {
      await transcribeUninstallModel(modelId);
      showSnackbar(t("settings.whisper.uninstallSuccess", { model: modelId }));
      await this.refreshModels();
    } catch (e) {
      showSnackbar(t("settings.whisper.uninstallFailed", { model: modelId, error: String(e) }), "error");
    } finally { run.finish(); }
  }

  openContextMenu(e: MouseEvent, model: { id: string; downloaded: boolean }) {
    e.preventDefault();
    e.stopPropagation();
    this.contextMenu = {
      x: e.clientX,
      y: e.clientY,
      kind: "model",
      modelId: model.id,
      downloaded: model.downloaded,
    };
  }

  openWhisperPanelContextMenu(e: MouseEvent) {
    e.preventDefault();
    this.contextMenu = {
      x: e.clientX,
      y: e.clientY,
      kind: "panel",
    };
  }

  closeContextMenu() {
    this.contextMenu = null;
  }

  // ─── Silero VAD add-ons (managed like the whisper models) ───────────────
  // Current whisper.cpp-compatible v6.2.0 model plus an optional arbitrary
  // local .bin. The active choice is persisted client-side
  // (`vesta-transcribe-vad-selection`) and read back by TranscribeTab when it
  // resolves which path to send to `transcribe_start`.

  async refreshAddons() {
    const request = this.addonRequests.begin();
    try {
      const s = await transcribeAddonsStatus();
      if (!request.isCurrent()) return;
      this.vadModels = s.vad_models;
      await this.refreshVadCustomValid();
    } catch (e) {
      console.error("Could not read transcription add-ons status:", e);
    }
  }

  async refreshVadCustomValid() {
    if (!this.vadSelection.customPath) {
      this.vadCustomValid = false;
      return;
    }
    try {
      this.vadCustomValid = await transcribePathExists(this.vadSelection.customPath);
    } catch {
      this.vadCustomValid = false;
    }
  }

  selectVadModel(modelId: string) {
    this.vadSelection = { modelId, customPath: null };
    saveVadSelection(this.vadSelection);
    window.dispatchEvent(new CustomEvent("vesta-vad-updated"));
  }

  handleVadModelClick(model: { id: string; downloaded: boolean }) {
    if (this.busy) return;
    if (model.downloaded) {
      this.selectVadModel(model.id);
      return;
    }
    void this.downloadVad(model.id);
  }

  async downloadVad(modelId: string) {
    const run = localWhisperOperation.begin("download-vad");
    if (!run) return false;
    this.downloadingVadId = modelId;
    try {
      await transcribeDownloadVad(modelId);
      await this.refreshAddons();
      if (run.isCancelled()) return false;
      if (!this.vadModels.some(model => model.id === modelId && model.downloaded)) throw new Error(t("setup.resourceCheckFailed"));
      this.selectVadModel(modelId);
      showSnackbar(t("settings.whisper.downloadSuccess", { model: `Silero Voice Activity Detection ${modelId}` }));
      return true;
    } catch (e) {
      const message = String(e).toLowerCase();
      if (message.includes("cancelled") || message.includes("canceled")) {
        snackbar.show(t("settings.modelDownloadCancelled", { model: `Silero Voice Activity Detection ${modelId}` }), "info");
      } else {
        showSnackbar(
          t("settings.whisper.downloadFailed", { model: `Silero Voice Activity Detection ${modelId}`, error: String(e) }),
          "error",
        );
      }
      return false;
    } finally {
      run.finish();
      this.downloadingVadId = null;
      this.progress = 0;
      this.progressMessage = "";
      this.progressStage = "";
    }
  }

  async uninstallVad(modelId: string) {
    const run = localWhisperOperation.begin("uninstall-vad");
    if (!run) return;
    try {
      await transcribeUninstallVad(modelId);
      await this.refreshAddons();
      if (!this.vadSelection.customPath && this.vadSelection.modelId === modelId) {
        this.selectVadModel(DEFAULT_VAD_MODEL_ID);
      }
      showSnackbar(t("settings.whisper.uninstallSuccess", { model: `Silero Voice Activity Detection ${modelId}` }));
    } catch (e) {
      showSnackbar(
        t("settings.whisper.uninstallFailed", { model: `Silero Voice Activity Detection ${modelId}`, error: String(e) }),
        "error",
      );
    } finally { run.finish(); }
  }

  async pickCustomVad() {
    const path = await guardedOpen({
      filters: [{ name: "Voice Activity Detection Model", extensions: ["bin"] }],
      multiple: false,
    });
    if (!path || typeof path !== "string") return;
    this.vadSelection = { modelId: this.vadSelection.modelId, customPath: path };
    saveVadSelection(this.vadSelection);
    await this.refreshVadCustomValid();
    window.dispatchEvent(new CustomEvent("vesta-vad-updated"));
  }

  clearCustomVad() {
    this.vadSelection = { modelId: this.vadSelection.modelId, customPath: null };
    saveVadSelection(this.vadSelection);
    this.vadCustomValid = false;
    window.dispatchEvent(new CustomEvent("vesta-vad-updated"));
  }

  /** Triggered from SettingsTab.svelte's shared reset-confirm dialog. */
  resetAll() {
    const baseModel = this.whisperModels.find((m) => m.id === "base");
    if (baseModel && baseModel.downloaded) {
      this.defaultWhisperModel = "base";
      vestaConfig.setItem("srt-default-whisper-model", "base");
      snackbar.show(t("settings.whisper.resetSuccess"), "info");
    } else {
      const alternate = this.whisperModels.find((m) => m.downloaded);
      if (alternate) {
        this.defaultWhisperModel = alternate.id;
        vestaConfig.setItem("srt-default-whisper-model", alternate.id);
        snackbar.show(t("settings.whisper.resetBaseNotDownloaded", { name: alternate.name }), "warning");
      } else {
        this.defaultWhisperModel = "base";
        vestaConfig.setItem("srt-default-whisper-model", "base");
        snackbar.show(t("settings.whisper.resetBaseDownloadWarning"), "warning");
      }
    }
  }
}

export const whisperModelsStore = new WhisperModelsStore();

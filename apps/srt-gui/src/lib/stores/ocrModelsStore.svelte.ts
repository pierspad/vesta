import { localOcrOperation, LatestRequest } from "$lib/utils/asyncControl.svelte";
import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import * as vestaConfig from "$lib/config/vestaConfig";
import {
  ocrListModels,
  ocrDownloadModel,
  ocrUninstallModel,
  ocrCancel,
  type OcrModel,
} from "$lib/services/ocr";

export type { OcrModel };

export const DEFAULT_OCR_MODEL_ID = "paddleocr-v4-mobile-multi";
const DEFAULT_OCR_MODEL_KEY = "vesta-default-ocr-model";

export const DEFAULT_OCR_MODELS: OcrModel[] = [
  { id: "paddleocr-v4-mobile-multi", name: "PaddleOCR v4 Mobile (Multilingual)", size: "~11 MB", downloaded: false },
  { id: "paddleocr-v4-mobile-cjk", name: "PaddleOCR v4 Mobile (CJK)", size: "~11 MB", downloaded: false },
  { id: "paddleocr-v4-mobile-latin", name: "PaddleOCR v4 Mobile (Latin)", size: "~10 MB", downloaded: false },
];

function showSnackbar(message: string, type: "error" | "success" = "success") {
  snackbar.show(message, type === "error" ? "error" : "success");
}

/** Local PaddleOCR model downloads, removal, and default selection. */
export class OcrModelsStore {
  private modelRequests = new LatestRequest();

  ocrModels = $state<OcrModel[]>(DEFAULT_OCR_MODELS.map((model) => ({ ...model })));
  downloadedOcrCount = $derived(this.ocrModels.filter((model) => model.downloaded).length);

  busy = $derived(localOcrOperation.busy);
  isDownloading = $state(false);
  isCancellingDownload = $state(false);
  downloadingModelId = $state<string | null>(null);
  pendingDefaultModelId = $state<string | null>(null);
  progress = $state(0);
  progressMessage = $state("");
  defaultOcrModel = $state(vestaConfig.getItem(DEFAULT_OCR_MODEL_KEY) || DEFAULT_OCR_MODEL_ID);

  async refreshModels() {
    const request = this.modelRequests.begin();
    try {
      const models = await ocrListModels();
      if (!request.isCurrent() || !Array.isArray(models)) return;
      this.ocrModels = models;
    } catch (e) {
      console.error("Could not list OCR models:", e);
    }
  }

  setDefaultModel(modelId: string, notify = true) {
    if (this.busy) return;
    this.defaultOcrModel = modelId;
    vestaConfig.setItem(DEFAULT_OCR_MODEL_KEY, modelId);
    if (notify) {
      const name = this.ocrModels.find((m) => m.id === modelId)?.name ?? modelId;
      showSnackbar(`${t("settings.defaultModel")}: ${name}`);
    }
  }

  handleModelClick(model: { id: string; downloaded: boolean }) {
    if (this.busy) return;
    if (model.downloaded) {
      this.setDefaultModel(model.id);
      return;
    }
    void this.downloadModel(model.id, true);
  }

  async downloadModel(modelId: string, setAsDefault = false) {
    const run = localOcrOperation.begin("download");
    if (!run) return false;
    this.isDownloading = true;
    this.isCancellingDownload = false;
    this.downloadingModelId = modelId;
    this.pendingDefaultModelId = setAsDefault ? modelId : null;
    try {
      const ok = await ocrDownloadModel(modelId);
      await this.refreshModels();

      const downloaded = this.ocrModels.find((m) => m.id === modelId)?.downloaded;
      if (run.isCancelled()) return false;
      if (!ok || !downloaded) throw new Error(t("setup.resourceCheckFailed"));
      if (this.pendingDefaultModelId === modelId) {
        this.defaultOcrModel = modelId;
        vestaConfig.setItem(DEFAULT_OCR_MODEL_KEY, modelId);
        showSnackbar(t("settings.whisper.downloadAndSetSuccess", { model: modelId }));
      } else {
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
    }
  }

  async cancelModelDownload() {
    if (!this.isDownloading || this.isCancellingDownload) return;
    this.isCancellingDownload = true;
    try {
      await localOcrOperation.cancel(ocrCancel);
    } catch (e) {
      showSnackbar(t("settings.whisper.cancelFailed", { error: String(e) }), "error");
      this.isCancellingDownload = false;
    }
  }

  async uninstallModel(modelId: string) {
    const run = localOcrOperation.begin("uninstall");
    if (!run) return;
    try {
      await ocrUninstallModel(modelId);
      showSnackbar(t("settings.whisper.uninstallSuccess", { model: modelId }));
      await this.refreshModels();
    } catch (e) {
      showSnackbar(t("settings.whisper.uninstallFailed", { model: modelId, error: String(e) }), "error");
    } finally {
      run.finish();
    }
  }

  resetAll() {
    const defaultModel = this.ocrModels.find((m) => m.id === DEFAULT_OCR_MODEL_ID);
    const nextId = defaultModel?.downloaded
      ? DEFAULT_OCR_MODEL_ID
      : (this.ocrModels.find((m) => m.downloaded)?.id ?? DEFAULT_OCR_MODEL_ID);
    this.defaultOcrModel = nextId;
    vestaConfig.setItem(DEFAULT_OCR_MODEL_KEY, nextId);
  }
}

export const ocrModelsStore = new OcrModelsStore();

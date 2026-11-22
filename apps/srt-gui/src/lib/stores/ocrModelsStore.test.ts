import { beforeEach, describe, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => new Map<string, string>());

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
}));

vi.mock("$lib/stores/snackbarStore.svelte", () => ({
  snackbar: { show: vi.fn() },
}));

vi.mock("$lib/services/ocr", () => ({
  ocrListModels: vi.fn(),
  ocrDownloadModel: vi.fn(),
  ocrUninstallModel: vi.fn(),
  ocrCancel: vi.fn(),
}));

import { OcrModelsStore, DEFAULT_OCR_MODEL_ID } from "./ocrModelsStore.svelte";
import { ocrDownloadModel, ocrListModels, ocrUninstallModel } from "$lib/services/ocr";

describe("OcrModelsStore", () => {
  beforeEach(() => {
    storage.clear();
    vi.clearAllMocks();
    vi.mocked(ocrListModels).mockResolvedValue([
      { id: "paddleocr-v4-mobile-multi", name: "Multi", size: "~11 MB", downloaded: true },
      { id: "paddleocr-v4-mobile-latin", name: "Latin", size: "~8 MB", downloaded: false },
    ]);
  });

  it("refreshes catalog and tracks downloaded count", async () => {
    const store = new OcrModelsStore();
    await store.refreshModels();
    expect(store.downloadedOcrCount).toBe(1);
  });

  it("downloads a model and sets default when verified", async () => {
    vi.mocked(ocrListModels).mockResolvedValueOnce([
      { id: "paddleocr-v4-mobile-multi", name: "Multi", size: "~11 MB", downloaded: true },
      { id: "paddleocr-v4-mobile-latin", name: "Latin", size: "~8 MB", downloaded: true },
    ]);
    vi.mocked(ocrDownloadModel).mockResolvedValueOnce(true);

    const store = new OcrModelsStore();
    const ok = await store.downloadModel("paddleocr-v4-mobile-latin", true);
    expect(ok).toBe(true);
    expect(store.defaultOcrModel).toBe("paddleocr-v4-mobile-latin");
    expect(storage.get("vesta-default-ocr-model")).toBe("paddleocr-v4-mobile-latin");
  });

  it("resets default selection to an installed model", async () => {
    const store = new OcrModelsStore();
    await store.refreshModels();
    store.setDefaultModel("paddleocr-v4-mobile-latin", false);
    store.resetAll();
    expect(store.defaultOcrModel).toBe(DEFAULT_OCR_MODEL_ID);
  });

  it("uninstalls a model and refreshes catalog", async () => {
    vi.mocked(ocrUninstallModel).mockResolvedValueOnce(true);
    const store = new OcrModelsStore();
    await store.uninstallModel("paddleocr-v4-mobile-multi");
    expect(ocrUninstallModel).toHaveBeenCalledWith("paddleocr-v4-mobile-multi");
    expect(ocrListModels).toHaveBeenCalled();
  });
});

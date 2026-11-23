// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";
import type { Tier } from "$lib/config/translationTiers";
import type { ApiKeyConfig } from "$lib/config/apiKeys";
import { aiStore } from "$lib/stores/aiStore.svelte";
import { DEFAULT_OCR_MODELS, ocrModelsStore } from "$lib/stores/ocrModelsStore.svelte";
import ExtractTab from "./ExtractTab.svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  notify: vi.fn(),
  listen: vi.fn(),
  loadTiers: vi.fn<() => Tier[]>(),
  loadIttTiers: vi.fn<() => Tier[]>(),
  loadAndValidateApiKeys: vi.fn<() => ApiKeyConfig[]>(),
  drop: null as null | ((paths: string[]) => void),
}));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: mocks.notify } }));
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: mocks.invoke }));
vi.mock("$lib/utils/defaultOutputDirectory", () => ({ defaultOutputDirectory: vi.fn().mockResolvedValue("/out") }));
vi.mock("$lib/utils/dialogGuard", () => ({ guardedOpen: vi.fn() }));
vi.mock("$lib/utils/dragDrop", () => ({ setupWebviewDragDrop: vi.fn(options => { mocks.drop = options.onDrop; return () => {}; }) }));
vi.mock("@tauri-apps/api/path", () => ({ join: vi.fn(async (...parts) => parts.join("/")) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("$lib/config/translationTiers", () => ({ loadTiers: mocks.loadTiers }));
vi.mock("$lib/config/ittTiers", () => ({ loadIttTiers: mocks.loadIttTiers }));
vi.mock("$lib/config/apiKeys", async (importOriginal) => {
  const actual = await importOriginal<typeof import("$lib/config/apiKeys")>();
  return { ...actual, loadAndValidateApiKeys: mocks.loadAndValidateApiKeys };
});

const originalShowModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
const mounted: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  mocks.loadTiers.mockReturnValue([]);
  mocks.loadIttTiers.mockReturnValue([]);
  mocks.loadAndValidateApiKeys.mockReturnValue([]);
  mocks.listen.mockResolvedValue(() => {});
  aiStore.killSwitchActive = false;
  aiStore.isOcring = false;
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value(this: HTMLDialogElement) { this.setAttribute("open", ""); },
  });
});

afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.replaceChildren();
  aiStore.killSwitchActive = false;
  aiStore.isOcring = false;
  ocrModelsStore.ocrModels = DEFAULT_OCR_MODELS.map((model) => ({ ...model }));
  ocrModelsStore.defaultOcrModel = "paddleocr-v4-mobile-multi";
  if (originalShowModal) Object.defineProperty(HTMLDialogElement.prototype, "showModal", originalShowModal);
  else Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  vi.clearAllMocks();
});

async function settle() { for (let i = 0; i < 6; i++) { await Promise.resolve(); await tick(); } }
async function render() {
  mocks.invoke.mockResolvedValue([]);
  const target = document.createElement("div"); document.body.append(target);
  mounted.push(mount(ExtractTab, { target, props: { active: true }, intro: false }));
  await settle(); mocks.drop!(["/film.mkv"]); await settle();
  return target;
}

function configureLlmTiers() {
  mocks.loadTiers.mockReturnValue([
    {
      id: "tier-1",
      entries: [{ id: "entry-1", provider: "google", apiKeyId: "key-1", model: "gemini-2.5-flash" }],
    },
  ]);
  mocks.loadAndValidateApiKeys.mockReturnValue([
    { id: "key-1", name: "Google", apiType: "google", apiKey: "AIza-test-key", capabilities: ["llm"] },
  ]);
}

const EXPECTED_TIERS = [
  [
    {
      provider: "google",
      model: "gemini-2.5-flash",
      api_key: "AIza-test-key",
      api_url: "https://generativelanguage.googleapis.com/v1beta",
      rpm: null,
      max_requests: null,
    },
  ],
];
it("ignores an obsolete output lookup failure after a newer lookup succeeds", async () => {
  const target = await render();
  let reject!: (error: unknown) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise((_, fail) => { reject = fail; })).mockResolvedValue([]);
  window.dispatchEvent(new Event("focus")); await settle();
  window.dispatchEvent(new Event("focus")); await settle();
  reject(new Error("obsolete lookup")); await settle();
  expect(target.textContent).not.toContain("obsolete lookup");
});
it("ignores a pending lookup from the previous media file", async () => {
  const target = await render();
  let reject!: (error: unknown) => void;
  mocks.invoke.mockImplementationOnce(() => new Promise((_, fail) => { reject = fail; })).mockResolvedValue([]);
  window.dispatchEvent(new Event("focus")); await settle();
  mocks.drop!(["/new-film.mkv"]); await settle();
  reject(new Error("previous media failure")); await settle();
  expect(target.textContent).not.toContain("previous media failure");
});
it("still displays current lookup errors", async () => {
  const target = await render();
  mocks.invoke.mockRejectedValueOnce(new Error("current lookup failure"));
  window.dispatchEvent(new Event("focus")); await settle();
  expect(target.textContent).toContain("current lookup failure");
});

it("does not notify when an in-flight extraction fails after unmount", async () => {
  const target = await render();
  const track = { index: 0, codec: "srt", language: "en", title: "English", text_based: true };
  let reject!: (error: unknown) => void;
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "extract_embedded_subtitle") return new Promise((_, fail) => { reject = fail; });
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-subtitle.mkv"]); await settle();
  const download = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  expect(download).not.toBeNull(); download!.click(); await settle();
  expect(reject).toBeTypeOf("function");
  await unmount(mounted.pop()!);
  reject(new Error("late extraction failure")); await settle();
  expect(mocks.notify).not.toHaveBeenCalled();
});

it("enables Save and Preview on a bitmap track and invokes ocr_embedded_subtitle when configured", async () => {
  configureLlmTiers();
  const target = await render();
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  let ocrDone = false;
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "existing_subtitle_outputs") return Promise.resolve(ocrDone ? ["with-pgs_en_2.srt"] : []);
    if (command === "ocr_embedded_subtitle") {
      ocrDone = true;
      return Promise.resolve(undefined);
    }
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]); await settle();

  const previewButton = target.querySelector<HTMLButtonElement>('button[title="View subtitles"]');
  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  expect(previewButton).not.toBeNull();
  expect(saveButton).not.toBeNull();
  expect(previewButton!.disabled).toBe(false);
  expect(saveButton!.disabled).toBe(false);

  saveButton!.click(); await settle();
  expect(mocks.invoke).toHaveBeenCalledWith("ocr_embedded_subtitle", {
    path: "/with-pgs.mkv",
    index: 2,
    outputPath: "/out/with-pgs_en_2.srt",
    language: "en",
    tiers: EXPECTED_TIERS,
  });
  expect(mocks.notify).toHaveBeenCalledWith("Saved: /out/with-pgs_en_2.srt", "success");
  expect(aiStore.isOcring).toBe(false);
  expect(target.querySelector('[aria-label="Already downloaded"]')).not.toBeNull();
});

it("shows ocrNoProvider error without invoking ocr_embedded_subtitle when no LLM tiers are configured", async () => {
  const target = await render();
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]); await settle();

  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  expect(saveButton).not.toBeNull();
  expect(saveButton!.disabled).toBe(false);
  saveButton!.click(); await settle();

  const expectedError = "Configure an LLM provider or local Ollama in Settings before running OCR.";
  expect(mocks.invoke).not.toHaveBeenCalledWith("ocr_embedded_subtitle", expect.anything());
  expect(target.textContent).toContain(expectedError);
  expect(mocks.notify).toHaveBeenCalledWith(expectedError, "error");
  expect(aiStore.isOcring).toBe(false);
});

it("disables Save and Preview on bitmap tracks when aiStore.killSwitchActive is true", async () => {
  configureLlmTiers();
  aiStore.killSwitchActive = true;
  const target = await render();
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]); await settle();

  const previewButton = target.querySelector<HTMLButtonElement>('button[title="View subtitles"]');
  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  expect(previewButton).not.toBeNull();
  expect(saveButton).not.toBeNull();
  expect(previewButton!.disabled).toBe(true);
  expect(saveButton!.disabled).toBe(true);
  expect(target.textContent).toContain("hdmv_pgs_subtitle · Image subtitles require OCR");
});

it("invokes preview_ocr_embedded_subtitle when clicking Preview on a bitmap track", async () => {
  configureLlmTiers();
  const target = await render();
  const track = { index: 3, codec: "hdmv_pgs_subtitle", language: "und", title: "", text_based: false };
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "preview_ocr_embedded_subtitle") return Promise.resolve("1\n00:00:01,000 --> 00:00:02,000\nHello OCR");
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]); await settle();

  const previewButton = target.querySelector<HTMLButtonElement>('button[title="View subtitles"]');
  expect(previewButton).not.toBeNull();
  previewButton!.click(); await settle();

  expect(mocks.invoke).toHaveBeenCalledWith("preview_ocr_embedded_subtitle", {
    path: "/with-pgs.mkv",
    index: 3,
    language: null,
    tiers: EXPECTED_TIERS,
  });
  expect(aiStore.isOcring).toBe(false);
  const dialog = target.querySelector("dialog");
  expect(dialog).not.toBeNull();
  const textarea = dialog!.querySelector("textarea");
  expect(textarea?.value).toContain("Hello OCR");
});

it("renders OCR progress, cancels via ocr_cancel, and cleans up aiStore.isOcring", async () => {
  configureLlmTiers();
  const target = await render();
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  let rejectOcr!: (error: unknown) => void;
  let progressHandler: ((event: { payload: unknown }) => void) | null = null;
  const unlisten = vi.fn();
  mocks.listen.mockImplementation(async (eventName: string, handler: (event: { payload: unknown }) => void) => {
    if (eventName === "ocr-progress") progressHandler = handler;
    return unlisten;
  });
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "ocr_embedded_subtitle") return new Promise((_, fail) => { rejectOcr = fail; });
    if (command === "ocr_cancel") return Promise.resolve(true);
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]); await settle();

  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  saveButton!.click(); await settle();
  expect(aiStore.isOcring).toBe(true);
  expect(aiStore.hasActiveAiProcess).toBe(true);
  expect(progressHandler).not.toBeNull();

  progressHandler!({ payload: { stage: "ocr", message: "Batch 1", percentage: 40, current: 2, total: 5 } });
  await settle();
  expect(target.textContent).toContain("Running OCR (2/5)…");
  expect(target.textContent).toContain("Batch 1");

  const cancelButton = [...target.querySelectorAll<HTMLButtonElement>("button")].find(b => b.textContent?.includes("Cancel OCR"));
  expect(cancelButton).toBeDefined();
  cancelButton!.click(); await settle();
  expect(mocks.invoke).toHaveBeenCalledWith("ocr_cancel");

  rejectOcr(new Error("OCR cancelled")); await settle();
  expect(mocks.notify).toHaveBeenCalledWith("OCR cancelled.", "info");
  expect(aiStore.isOcring).toBe(false);
  expect(aiStore.hasActiveAiProcess).toBe(false);
  expect(unlisten).toHaveBeenCalledOnce();
});

it("allows bitmap OCR using a downloaded local OCR model even when aiStore.killSwitchActive is true", async () => {
  aiStore.killSwitchActive = true;
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "ocr_list_models") {
      return Promise.resolve([
        { id: "paddleocr-v4-mobile-multi", name: "PaddleOCR v4 Mobile (Multilingual)", size: "~11 MB", downloaded: true },
      ]);
    }
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "ocr_embedded_subtitle") return Promise.resolve(undefined);
    return Promise.resolve([]);
  });
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(ExtractTab, { target, props: { active: true }, intro: false }));
  await settle();
  mocks.drop!(["/with-pgs.mkv"]);
  await settle();

  const previewButton = target.querySelector<HTMLButtonElement>('button[title="View subtitles"]');
  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  expect(previewButton).not.toBeNull();
  expect(saveButton).not.toBeNull();
  expect(previewButton!.disabled).toBe(false);
  expect(saveButton!.disabled).toBe(false);

  saveButton!.click();
  await settle();
  expect(mocks.invoke).toHaveBeenCalledWith("ocr_embedded_subtitle", {
    path: "/with-pgs.mkv",
    index: 2,
    outputPath: "/out/with-pgs_en_2.srt",
    language: "en",
    tiers: [
      [
        {
          provider: "local_ocr",
          model: "paddleocr-v4-mobile-multi",
          api_key: null,
          api_url: null,
          rpm: null,
          max_requests: null,
        },
      ],
    ],
  });
  expect(mocks.notify).toHaveBeenCalledWith("Saved: /out/with-pgs_en_2.srt", "success");
});

it("hides hardcoded mode switch and returns to embedded when aiStore.killSwitchActive is enabled", async () => {
  configureLlmTiers();
  const target = await render();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([]);
    if (command === "ocr_video_preview_frame") {
      return Promise.resolve({
        data_url: "data:image/jpeg;base64,AA==",
        width: 1280,
        height: 720,
        duration_ms: 120000,
        time_ms: 30000,
      });
    }
    return Promise.resolve([]);
  });
  mocks.drop!(["/film.mp4"]);
  await settle();

  const hardcodedTab = target.querySelector<HTMLButtonElement>('button[role="tab"][aria-selected="false"]');
  expect(hardcodedTab).not.toBeNull();
  expect(hardcodedTab!.textContent?.trim()).toBe("Hardcoded");
  hardcodedTab!.click();
  await settle();
  expect(target.querySelector("#hardcoded-roi-panel")).not.toBeNull();

  const lockButton = target.querySelector<HTMLButtonElement>('button[aria-label="Keep aspect ratio"]');
  expect(lockButton).not.toBeNull();
  expect(target.querySelectorAll(".number-control-buttons")).toHaveLength(4);
  lockButton!.click();
  await settle();
  expect(lockButton!.getAttribute("aria-pressed")).toBe("true");

  aiStore.killSwitchActive = true;
  await settle();

  expect(target.querySelector("#hardcoded-roi-panel")).toBeNull();
  expect(target.querySelector('[role="tablist"][aria-label="Extract subtitles"]')).toBeNull();
  const fallbackHardcodedBtn = [...target.querySelectorAll<HTMLButtonElement>("button")].find(
    (b) => b.textContent?.trim() === "Hardcoded",
  );
  expect(fallbackHardcodedBtn).toBeUndefined();

  aiStore.killSwitchActive = false;
  await settle();
  expect(target.querySelector('[role="tablist"][aria-label="Extract subtitles"]')).not.toBeNull();
});

it("loads preview frame, previews ROI OCR, and runs hardcoded ROI extraction", async () => {
  configureLlmTiers();
  const target = await render();
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "ocr_list_models") return Promise.resolve([]);
    if (command === "embedded_subtitle_tracks") return Promise.resolve([]);
    if (command === "ocr_video_preview_frame") {
      return Promise.resolve({
        data_url: "data:image/jpeg;base64,AA==",
        width: 1280,
        height: 720,
        duration_ms: 120000,
        time_ms: 30000,
      });
    }
    if (command === "preview_ocr_hardcoded_subtitle") {
      return Promise.resolve("1\n00:00:01,000 --> 00:00:03,000\nBurned subtitle");
    }
    if (command === "ocr_hardcoded_subtitle") return Promise.resolve(undefined);
    return Promise.resolve([]);
  });
  mocks.drop!(["/film.mp4"]);
  await settle();

  const hardcodedTab = target.querySelector<HTMLButtonElement>('button[role="tab"][aria-selected="false"]');
  expect(hardcodedTab).not.toBeNull();
  hardcodedTab!.click();
  await settle();

  expect(mocks.invoke).toHaveBeenCalledWith("ocr_video_preview_frame", {
    path: "/film.mp4",
    timeMs: null,
  });
  const img = target.querySelector<HTMLImageElement>('img[alt="Hardcoded subtitles"]');
  expect(img).not.toBeNull();
  expect(img!.getAttribute("src")).toBe("data:image/jpeg;base64,AA==");
  expect(target.querySelector('[data-testid="roi-overlay"]')).not.toBeNull();
  expect(target.querySelectorAll('[data-testid="roi-overlay"] button')).toHaveLength(8);

  const fpsInput = target.querySelector<HTMLInputElement>('input[aria-label="Sample rate (FPS)"]');
  expect(fpsInput).not.toBeNull();
  fpsInput!.focus();
  await settle();
  const fpsOption3 = [...document.body.querySelectorAll<HTMLButtonElement>(".searchable-select-option")].find(
    (opt) => opt.textContent?.trim() === "3 FPS",
  );
  expect(fpsOption3).toBeDefined();
  fpsOption3!.click();
  await settle();

  const previewRoiButton = [...target.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
    b.textContent?.includes("Preview area OCR"),
  );
  expect(previewRoiButton).toBeDefined();
  previewRoiButton!.click();
  await settle();

  expect(mocks.invoke).toHaveBeenCalledWith("preview_ocr_hardcoded_subtitle", {
    path: "/film.mp4",
    roi: { x: 0.08, y: 0.74, width: 0.84, height: 0.2 },
    sampleFps: 3,
    language: null,
    tiers: EXPECTED_TIERS,
  });
  const dialog = target.querySelector("dialog");
  expect(dialog).not.toBeNull();
  expect(dialog!.querySelector("textarea")?.value).toContain("Burned subtitle");

  const closeDialogButton = dialog!.querySelector<HTMLButtonElement>("button.close-button");
  closeDialogButton!.click();
  await settle();

  const extractRoiButton = [...target.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
    b.textContent?.includes("Extract area to SRT"),
  );
  expect(extractRoiButton).toBeDefined();
  extractRoiButton!.click();
  await settle();

  expect(mocks.invoke).toHaveBeenCalledWith("ocr_hardcoded_subtitle", {
    path: "/film.mp4",
    outputPath: "/out/film_hardcoded.srt",
    roi: { x: 0.08, y: 0.74, width: 0.84, height: 0.2 },
    sampleFps: 3,
    language: null,
    tiers: EXPECTED_TIERS,
  });
  expect(mocks.notify).toHaveBeenCalledWith("Saved: /out/film_hardcoded.srt", "success");
});

it("prefers dedicated ITT tiers over translation LLM tiers when configured", async () => {
  configureLlmTiers();
  mocks.loadIttTiers.mockReturnValue([
    {
      id: "itt-tier-1",
      entries: [{ id: "itt-entry-1", provider: "custom", apiKeyId: "local-itt", model: "qwen2.5vl:7b" }],
    },
  ]);
  mocks.loadAndValidateApiKeys.mockReturnValue([
    { id: "key-1", name: "Google", apiType: "google", apiKey: "AIza-test-key", capabilities: ["llm"] },
    { id: "local-itt", name: "Local ITT", apiType: "custom", apiKey: "", apiUrl: "http://localhost:11434/v1", capabilities: ["itt"] },
  ]);
  const target = await render();
  const track = { index: 2, codec: "hdmv_pgs_subtitle", language: "eng", title: "English PGS", text_based: false };
  mocks.invoke.mockImplementation((command: string) => {
    if (command === "embedded_subtitle_tracks") return Promise.resolve([track]);
    if (command === "ocr_embedded_subtitle") return Promise.resolve(undefined);
    return Promise.resolve([]);
  });
  mocks.drop!(["/with-pgs.mkv"]);
  await settle();

  const saveButton = target.querySelector<HTMLButtonElement>('button[aria-busy="false"]');
  saveButton!.click();
  await settle();

  expect(mocks.invoke).toHaveBeenCalledWith("ocr_embedded_subtitle", {
    path: "/with-pgs.mkv",
    index: 2,
    outputPath: "/out/with-pgs_en_2.srt",
    language: "en",
    tiers: [
      [
        {
          provider: "custom",
          model: "qwen2.5vl:7b",
          api_key: null,
          api_url: "http://localhost:11434/v1",
          rpm: null,
          max_requests: null,
        },
      ],
    ],
  });
});

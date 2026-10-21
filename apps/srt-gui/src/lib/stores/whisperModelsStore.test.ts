import { afterEach, beforeEach, expect, it, vi } from "vitest";
vi.mock("$lib/services/transcribe", () => ({ transcribeListModels: vi.fn(), transcribeDownloadModel: vi.fn(), transcribeCancel: vi.fn(), transcribeUninstallModel: vi.fn(), transcribeAddonsStatus: vi.fn(), transcribePathExists: vi.fn(), transcribeDownloadVad: vi.fn(), transcribeUninstallVad: vi.fn() }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { transcribeDownloadModel, transcribeCancel, transcribeListModels, transcribeUninstallModel } from "$lib/services/transcribe";
import { WhisperModelsStore } from "./whisperModelsStore.svelte";
import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => resolve = done); return { promise, resolve }; }
const model = (id: string) => ({ id, name: id, size: "", speed: "", downloaded: true });
beforeEach(() => {
 vi.clearAllMocks(); vi.stubGlobal("window", { dispatchEvent: vi.fn() });
 vi.stubGlobal("CustomEvent", class { constructor(public type: string, public options?: unknown) {} });
 vi.mocked(transcribeListModels).mockResolvedValue([model("small")]);
});
afterEach(() => vi.unstubAllGlobals());
it("serializes downloads and sets the default only after verified installation", async () => {
 const store = new WhisperModelsStore(); const download = deferred<boolean>(); vi.mocked(transcribeDownloadModel).mockReturnValueOnce(download.promise);
 const first = store.downloadModel("small", true); expect(await store.downloadModel("large")).toBe(false);
 expect(transcribeDownloadModel).toHaveBeenCalledOnce(); download.resolve(true); expect(await first).toBe(true);
 expect(store.defaultWhisperModel).toBe("small"); expect(store.busy).toBe(false);
});
it("retains the shared lock until cancellation settles and does not select a cancelled download", async () => {
 const store = new WhisperModelsStore(); const download = deferred<boolean>(); const cancellation = deferred<void>();
 vi.mocked(transcribeDownloadModel).mockReturnValueOnce(download.promise); vi.mocked(transcribeCancel).mockReturnValueOnce(cancellation.promise);
 const first = store.downloadModel("small", true); const cancel = store.cancelModelDownload();
 download.resolve(true); expect(await first).toBe(false); expect(store.busy).toBe(true);
 expect(await store.downloadModel("large")).toBe(false); expect(store.defaultWhisperModel).toBe("base");
 cancellation.resolve(); await cancel; expect(store.busy).toBe(false);
});
it("cannot uninstall a model during inference", async () => {
 const run = localWhisperOperation.begin("transcription")!; const store = new WhisperModelsStore();
 try { await store.uninstallModel("small"); expect(transcribeUninstallModel).not.toHaveBeenCalled(); }
 finally { run.finish(); }
});
it("an older discovery response cannot restore outdated installation status", async () => {
 const store = new WhisperModelsStore(); const first = deferred<ReturnType<typeof model>[]>(); const second = deferred<ReturnType<typeof model>[]>();
 vi.mocked(transcribeListModels).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
 const old = store.refreshModels(); const current = store.refreshModels(); second.resolve([model("medium")]); await current;
 first.resolve([model("small")]); await old; expect(store.whisperModels.map(model => model.id)).toEqual(["medium"]);
});

it("switches the selected Silero model to persistent WebRTC after uninstall", async () => {
 const { transcribeUninstallVad, transcribeAddonsStatus } = await import("$lib/services/transcribe");
 const { loadVadSelection } = await import("$lib/config/vadSelection");
 const store = new WhisperModelsStore(); store.selectVadModel("v6.2.0");
 vi.mocked(transcribeUninstallVad).mockResolvedValue(true);
 vi.mocked(transcribeAddonsStatus).mockResolvedValue({ vad_models: [] } as any);
 await store.uninstallVad("v6.2.0");
 expect(store.vadSelection).toEqual({ modelId: "webrtc", customPath: null });
 expect(loadVadSelection()).toEqual(store.vadSelection);
});
it("keeps the selected VAD when uninstall fails", async () => {
 const { transcribeUninstallVad } = await import("$lib/services/transcribe");
 const store = new WhisperModelsStore(); store.selectVadModel("v6.2.0");
 vi.mocked(transcribeUninstallVad).mockRejectedValueOnce(new Error("disk error"));
 await store.uninstallVad("v6.2.0");
 expect(store.vadSelection.modelId).toBe("v6.2.0");
});
it("preserves a custom VAD when removing an unused built-in model", async () => {
 const { transcribeUninstallVad, transcribeAddonsStatus } = await import("$lib/services/transcribe");
 const store = new WhisperModelsStore(); store.vadSelection = { modelId: "v6.2.0", customPath: "/custom.bin" };
 vi.mocked(transcribeUninstallVad).mockResolvedValue(true);
 vi.mocked(transcribeAddonsStatus).mockResolvedValue({ vad_models: [] } as any);
 await store.uninstallVad("v6.2.0");
 expect(store.vadSelection.customPath).toBe("/custom.bin");
});

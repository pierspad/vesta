import { expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ models: vi.fn(), backends: vi.fn(), addons: vi.fn(), exists: vi.fn(), selection: vi.fn() }));
vi.mock("$lib/services/transcribe", () => ({ transcribeListModels: mocks.models, transcribeCheckBackends: mocks.backends, transcribeAddonsStatus: mocks.addons, transcribePathExists: mocks.exists }));
vi.mock("$lib/config/vadSelection", () => ({ loadVadSelection: mocks.selection }));
import { TranscriptionResources } from "./transcriptionResources.svelte";
it("ignores out-of-order model discovery and late updates after disposal", async () => {
 mocks.selection.mockReturnValue({ modelId: "silero", customPath: null });
 let resolveOld!: (value: unknown) => void;
 mocks.models.mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; })).mockResolvedValueOnce([{ id: "new", downloaded: true }]);
 const state = new TranscriptionResources(); const old = state.refreshModels(); await state.refreshModels();
 resolveOld([{ id: "old" }]); await old; expect(state.models[0].id).toBe("new");
 state.dispose(); mocks.models.mockResolvedValue([{ id: "disposed" }]); await state.refreshModels(); expect(state.models[0].id).toBe("new");
});
it("checks the selected custom VAD file instead of assuming an installed built-in model is sufficient", async () => {
 mocks.selection.mockReturnValue({ modelId: "silero", customPath: "/missing.bin" });
 mocks.addons.mockResolvedValue({ gpu_supported: true, vad_models: [{ id: "silero", downloaded: true }] }); mocks.exists.mockResolvedValue(false);
 const state = new TranscriptionResources(); await state.refreshAddons();
 expect(mocks.exists).toHaveBeenCalledWith("/missing.bin"); expect(state.vadInstalled).toBe(false); expect(state.gpuSupported).toBe(true);
});

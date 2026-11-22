import { beforeEach, expect, it, vi } from "vitest";
vi.mock("./tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("./transcribe", () => ({ transcribeListModels: vi.fn(), transcribeAddonsStatus: vi.fn(), transcribeDownloadModel: vi.fn(), transcribeDownloadVad: vi.fn() }));
vi.mock("./media", () => ({ downloadFfmpeg: vi.fn() }));
import { invokeCommand } from "./tauriClient";
import { transcribeAddonsStatus, transcribeListModels } from "./transcribe";
import { installSetupResource, loadSetupResources } from "./setup";
beforeEach(() => vi.resetAllMocks());
it("does not hide discovery failures behind a ready-looking setup", async () => {
 vi.mocked(transcribeListModels).mockRejectedValue(new Error("model discovery failed"));
 vi.mocked(transcribeAddonsStatus).mockResolvedValue({ vad_models: [], gpu_supported: false });
 vi.mocked(invokeCommand).mockResolvedValue([]);
 await expect(loadSetupResources()).rejects.toThrow("model discovery failed");
});
it("requires both media tools and exposes the archive size from the backend", async () => {
 vi.mocked(transcribeListModels).mockResolvedValue([]);
 vi.mocked(transcribeAddonsStatus).mockResolvedValue({ vad_models: [], gpu_supported: false });
 vi.mocked(invokeCommand).mockImplementation(async command => command === "setup_media_status" ? { ready: false, download_bytes: 80_000_000 } : []);
 expect(await loadSetupResources()).toMatchObject({ mediaReady: false, mediaDownloadBytes: 80_000_000 });
});
it("treats a false font installation result as a failure", async () => {
 vi.mocked(invokeCommand).mockResolvedValue(false);
 await expect(installSetupResource({ id: "font", kind: "font", resourceId: "jp", name: "Japanese", size: "~12 MB", installed: false })).rejects.toThrow("Font installation failed");
});

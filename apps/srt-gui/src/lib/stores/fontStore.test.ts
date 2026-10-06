import { beforeEach, describe, expect, it, vi } from "vitest";
import type { FontStatusInfo } from "./fontStore.svelte";
vi.mock("$lib/services/tauriClient", () => ({ invokeCommand: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { fontStore } from "./fontStore.svelte";
const font = (id: string, downloaded = false): FontStatusInfo => ({
  id, downloaded, name: id, filename: id, language_name: id,
  target_languages: [], approx_size: "1 MB",
});
describe("bulk font downloads", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    fontStore.fonts = [font("cached", true), font("first"), font("second")];
    fontStore.downloadingFontId = null;
    fontStore.downloadingAll = false;
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === "flashcard_list_fonts" ? fontStore.fonts : true,
    );
  });
  it("downloads only missing fonts and releases the lock", async () => {
    await fontStore.downloadAllFonts();
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === "flashcard_download_font"))
      .toEqual([["flashcard_download_font", { fontId: "first" }], ["flashcard_download_font", { fontId: "second" }]]);
    expect(fontStore.downloadingAll).toBe(false);
    expect(fontStore.downloadingFontId).toBeNull();
  });
  it("continues after a failed download", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("offline"));
    await fontStore.downloadAllFonts();
    expect(invoke).toHaveBeenCalledWith("flashcard_download_font", { fontId: "second" });
    expect(fontStore.downloadingAll).toBe(false);
  });
  it("rejects additional downloads while a batch is running", async () => {
    fontStore.downloadingAll = true;
    await fontStore.downloadAllFonts();
    expect(await fontStore.downloadFont("first")).toBe(false);
    expect(invoke).not.toHaveBeenCalled();
  });
});

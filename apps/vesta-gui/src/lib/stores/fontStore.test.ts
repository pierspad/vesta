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

it("a false native download result is not reported as successful", async () => {
 vi.mocked(invoke).mockReset(); vi.mocked(invoke).mockResolvedValue(false);
 expect(await fontStore.downloadFont("first")).toBe(false);
 expect(fontStore.downloadingFontId).toBeNull();
});
it("cannot delete a font while a batch is using it", async () => {
 vi.mocked(invoke).mockReset(); fontStore.downloadingAll = true;
 try { expect(await fontStore.deleteFont("first")).toBe(false); expect(invoke).not.toHaveBeenCalled(); }
 finally { fontStore.downloadingAll = false; }
});

describe("single font download, delete, and catalog load", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    fontStore.fonts = [font("cached", true), font("noto-sans-jp", false)];
    fontStore.downloadingFontId = null;
    fontStore.downloadingAll = false;
    fontStore.deletingFontId = null;
  });

  it("downloads a font, refreshes the list, and clears downloadingFontId", async () => {
    const updated = [font("cached", true), font("noto-sans-jp", true)];
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === "flashcard_list_fonts" ? updated : true,
    );

    expect(await fontStore.downloadFont("noto-sans-jp")).toBe(true);
    expect(invoke).toHaveBeenCalledWith("flashcard_download_font", { fontId: "noto-sans-jp" });
    expect(invoke).toHaveBeenCalledWith("flashcard_list_fonts");
    expect(fontStore.fonts.find((f) => f.id === "noto-sans-jp")?.downloaded).toBe(true);
    expect(fontStore.downloadingFontId).toBeNull();
  });

  it("deletes a font, refreshes the list, and handles delete failures cleanly", async () => {
    const updated = [font("cached", false), font("noto-sans-jp", false)];
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === "flashcard_list_fonts" ? updated : true,
    );

    expect(await fontStore.deleteFont("cached")).toBe(true);
    expect(invoke).toHaveBeenCalledWith("flashcard_delete_font", { fontId: "cached" });
    expect(fontStore.deletingFontId).toBeNull();
    expect(fontStore.fonts[0].downloaded).toBe(false);

    vi.mocked(invoke).mockRejectedValueOnce(new Error("permission denied"));
    expect(await fontStore.deleteFont("cached")).toBe(false);
    expect(fontStore.deletingFontId).toBeNull();
  });

  it("reports false when flashcard_list_fonts fails during loadFonts", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("backend unavailable"));
    expect(await fontStore.loadFonts()).toBe(false);
  });
});

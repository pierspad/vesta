import { afterEach, expect, it, vi } from "vitest";
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { writeClipboard, readClipboard } from "./clipboard";
afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); });
it("only reports copy success after the clipboard accepts the write", async () => {
 const writeText = vi.fn().mockResolvedValue(undefined); vi.stubGlobal("navigator", { clipboard: { writeText } });
 expect(await writeClipboard("text", "Copied")).toBe(true); expect(writeText).toHaveBeenCalledWith("text");
 expect(snackbar.show).toHaveBeenCalledWith("Copied", "success");
});
it("returns false on denied copy so cut operations can preserve text", async () => {
 vi.stubGlobal("navigator", { clipboard: { writeText: vi.fn().mockRejectedValue(new Error("denied")) } });
 expect(await writeClipboard("secret", "Copied")).toBe(false); expect(snackbar.show).toHaveBeenCalledOnce();
 expect(snackbar.show).toHaveBeenCalledWith(expect.any(String), "error"); expect(snackbar.show).not.toHaveBeenCalledWith("Copied", "success");
});
it("distinguishes an empty clipboard from clipboard access failure", async () => {
 vi.stubGlobal("navigator", { clipboard: { readText: vi.fn().mockResolvedValue("") } }); expect(await readClipboard()).toBe("");
 vi.stubGlobal("navigator", {}); expect(await readClipboard()).toBeNull();
});

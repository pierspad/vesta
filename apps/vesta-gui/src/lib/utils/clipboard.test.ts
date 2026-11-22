const native = vi.hoisted(() => ({ enabled: false, write: vi.fn(), read: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => native.enabled }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: native.write, readText: native.read }));
import { afterEach, expect, it, vi } from "vitest";
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { writeClipboard, readClipboard } from "./clipboard";
afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); native.enabled = false; });
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

it("uses the native clipboard even when the WebView clipboard is unavailable", async () => {
 native.enabled = true;
 native.write.mockResolvedValue(undefined); native.read.mockResolvedValue("native text");
 vi.stubGlobal("navigator", {});
 expect(await writeClipboard("path", "Copied")).toBe(true);
 expect(native.write).toHaveBeenCalledWith("path");
 expect(await readClipboard()).toBe("native text");
});
it("reports a native rejection without claiming success or falling back to the browser", async () => {
 native.enabled = true; native.write.mockRejectedValue(new Error("busy"));
 const browserWrite = vi.fn(); vi.stubGlobal("navigator", { clipboard: { writeText: browserWrite } });
 expect(await writeClipboard("path", "Copied")).toBe(false);
 expect(browserWrite).not.toHaveBeenCalled();
 expect(snackbar.show).toHaveBeenCalledWith(expect.any(String), "error");
});

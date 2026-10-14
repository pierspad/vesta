import { afterEach, expect, it, vi } from "vitest";
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { parentPath, validatePickerPath } from "./pathValidation";
afterEach(() => vi.clearAllMocks());
it("validates output parents including Windows drives, UNC paths and filesystem root", () => {
 expect(parentPath("/subs.srt")).toBe("/"); expect(parentPath("C:\\subs.srt")).toBe("C:\\");
 expect(parentPath("\\\\server\\share\\subs.srt")).toBe("\\\\server\\share"); expect(parentPath("C:/subtitles/subs.srt")).toBe("C:/subtitles");
});
it("handles invalid, absent and unreadable paths consistently", async () => {
 const exists = vi.fn().mockResolvedValue(false);
 expect(await validatePickerPath(" ", "input", exists)).toBeNull(); expect(exists).not.toHaveBeenCalled();
 expect(await validatePickerPath(" film.mkv ", "input", exists)).toBeNull(); expect(exists).toHaveBeenCalledWith("film.mkv");
 exists.mockRejectedValue(new Error("permission denied")); expect(await validatePickerPath("/out/subs.srt", "output", exists)).toBeNull();
 expect(snackbar.show).toHaveBeenLastCalledWith("Error: permission denied", "error");
});
it("returns valid paths unchanged and checks only the output directory", async () => {
 const exists = vi.fn().mockResolvedValue(true);
 expect(await validatePickerPath(" /out/new.srt ", "output", exists)).toBe("/out/new.srt");
 expect(exists).toHaveBeenCalledExactlyOnceWith("/out");
});

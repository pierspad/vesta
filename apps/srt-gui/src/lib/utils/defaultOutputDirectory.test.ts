import { beforeEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ downloadDir: vi.fn(), homeDir: vi.fn(), invoke: vi.fn() }));
vi.mock("@tauri-apps/api/path", () => mocks);
vi.mock("@tauri-apps/api/core", () => mocks);
import { defaultOutputDirectory } from "./defaultOutputDirectory";
beforeEach(() => {
  vi.resetAllMocks();
  mocks.downloadDir.mockResolvedValue("/home/user/Downloads");
  mocks.homeDir.mockResolvedValue("/home/user");
  mocks.invoke.mockResolvedValue(true);
});
it("uses the platform Downloads folder when it exists", async () => {
  expect(await defaultOutputDirectory()).toBe("/home/user/Downloads");
  expect(mocks.homeDir).not.toHaveBeenCalled();
});
it("falls back when Downloads is missing", async () => {
  mocks.invoke.mockResolvedValue(false);
  expect(await defaultOutputDirectory()).toBe("/home/user");
});
it("falls back when platform path resolution fails", async () => {
  mocks.downloadDir.mockRejectedValue(new Error("unavailable"));
  expect(await defaultOutputDirectory()).toBe("/home/user");
});

import { beforeEach, expect, it, vi } from "vitest";
vi.mock("$lib/stores/snackbarStore.svelte", () => ({ snackbar: { show: vi.fn() } }));
vi.mock("$lib/i18n", () => ({ t: (key: string) => key }));
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { invokeCommand } from "$lib/services/tauriClient";
import * as config from "./vestaConfig";
const invoke = vi.mocked(invokeCommand);
beforeEach(async () => { invoke.mockResolvedValue(undefined); await config.replaceAll({}); invoke.mockClear(); });
it("flush waits for durable writes before setup can reload", async () => {
  let release!: () => void;
  invoke.mockImplementationOnce(() => new Promise<void>((resolve) => release = resolve));
  config.setItem("vesta-first-run-setup-complete", "true");
  let completed = false;
  const saving = config.flush().then(() => completed = true);
  await Promise.resolve(); await Promise.resolve();
  expect(completed).toBe(false);
  expect(invoke).toHaveBeenCalledTimes(1);
  release(); await saving;
  expect(invoke).toHaveBeenLastCalledWith("config_replace_all", { values: { "vesta-first-run-setup-complete": "true" } });
});
it("flush exposes a disk failure instead of reporting setup complete", async () => {
  invoke.mockRejectedValueOnce(new Error("disk full"));
  await expect(config.flush()).rejects.toThrow("disk full");
});

it("reports fire-and-forget preference failures and keeps the session value", async () => {
 invoke.mockRejectedValueOnce(new Error("disk full"));
 config.setItem("example-preference", "value");
 await vi.waitFor(() => expect(snackbar.show).toHaveBeenCalledWith("common.settingsNotSaved", "error"));
 expect(config.getItem("example-preference")).toBe("value");
 await config.flush();
 expect(invoke).toHaveBeenLastCalledWith("config_replace_all", { values: { "example-preference": "value" } });
});

it("serializes installation reset after pending writes and clears legacy preferences", async () => {
  const legacyClear = vi.fn();
  const sessionClear = vi.fn();
  vi.stubGlobal("localStorage", { clear: legacyClear });
  vi.stubGlobal("sessionStorage", { clear: sessionClear });
  let release!: () => void;
  invoke.mockImplementationOnce(() => new Promise<void>((resolve) => release = resolve));
  config.setItem("preference", "old");
  const reset = config.resetInstallation();
  await Promise.resolve(); await Promise.resolve();
  expect(invoke).toHaveBeenCalledTimes(1);
  release();
  await reset;
  expect(invoke).toHaveBeenLastCalledWith("config_reset_installation", {});
  expect(config.exportSnapshot()).toEqual({});
  expect(legacyClear).toHaveBeenCalledOnce();
  expect(sessionClear).toHaveBeenCalledOnce();
  vi.unstubAllGlobals();
});

it("keeps preferences when installation reset fails", async () => {
  config.setItem("preference", "keep");
  await config.flush();
  invoke.mockRejectedValueOnce(new Error("cache locked"));
  await expect(config.resetInstallation()).rejects.toThrow("cache locked");
  expect(config.getItem("preference")).toBe("keep");
});

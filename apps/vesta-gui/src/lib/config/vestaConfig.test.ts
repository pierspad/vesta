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

it("retries incomplete localStorage migration on next startup until atomic write succeeds", async () => {
  vi.resetModules();
  const mockInvoke = vi.fn();
  vi.doMock("$lib/services/tauriClient", () => ({ invokeCommand: mockInvoke }));
  vi.stubGlobal("localStorage", {
    length: 2,
    key: (index: number) => (index === 0 ? "api-key" : index === 1 ? "language" : null),
    getItem: (key: string) => (key === "api-key" ? "secret" : key === "language" ? "it" : null),
    clear: vi.fn(),
  });

  try {
    mockInvoke
      .mockResolvedValueOnce({ entries: {}, recovered_backup_path: null })
      .mockRejectedValueOnce(new Error("transient disk failure"));

    const firstBoot = await import("./vestaConfig");
    await firstBoot.hydrate();

    expect(firstBoot.getItem("api-key")).toBe("secret");
    expect(firstBoot.getItem("language")).toBe("it");
    expect(firstBoot.exportSnapshot()).not.toHaveProperty("__vesta_config_migrated_from_local_storage__");

    vi.resetModules();
    vi.doMock("$lib/services/tauriClient", () => ({ invokeCommand: mockInvoke }));
    mockInvoke
      .mockResolvedValueOnce({ entries: { language: "fr" }, recovered_backup_path: null })
      .mockResolvedValueOnce(undefined);

    const secondBoot = await import("./vestaConfig");
    await secondBoot.hydrate();

    expect(secondBoot.getItem("api-key")).toBe("secret");
    expect(secondBoot.getItem("language")).toBe("fr");
    expect(mockInvoke).toHaveBeenLastCalledWith("config_replace_all", {
      values: {
        language: "fr",
        "api-key": "secret",
        __vesta_config_migrated_from_local_storage__: "true",
      },
    });
  } finally {
    vi.doUnmock("$lib/services/tauriClient");
    vi.unstubAllGlobals();
  }
});

it("records and consumes recovered backup paths without treating recovery as a fresh install", async () => {
  vi.resetModules();
  const mockInvoke = vi.fn().mockResolvedValueOnce({
    entries: {},
    recovered_backup_path: "/home/user/.config/vesta/vesta_config.json.corrupt.123",
  });
  vi.doMock("$lib/services/tauriClient", () => ({ invokeCommand: mockInvoke }));
  vi.stubGlobal("localStorage", { length: 0, key: () => null, getItem: () => null, clear: vi.fn() });

  try {
    const freshModule = await import("./vestaConfig");
    await freshModule.hydrate();

    expect(freshModule.isNewInstallation()).toBe(false);
    expect(freshModule.consumeRecoveredBackupPath()).toBe(
      "/home/user/.config/vesta/vesta_config.json.corrupt.123",
    );
    expect(freshModule.consumeRecoveredBackupPath()).toBeNull();
  } finally {
    vi.doUnmock("$lib/services/tauriClient");
    vi.unstubAllGlobals();
  }
});

it("migrates legacy srt-* keys to vesta-* equivalents, preserves existing vesta-* values, and is a no-op once clean", async () => {
  vi.resetModules();
  const mockInvoke = vi.fn()
    .mockResolvedValueOnce({
      entries: {
        "srt-tools-api-keys": "[{\"id\":\"legacy\"}]",
        "srt-tools-custom-models": "[{\"id\":\"custom\"}]",
        "srt-default-whisper-model": "small",
        "srt-tools-ui-language": "it",
        "srt-tools-shortcut-overrides": "{\"sync-new\":\"Ctrl+K\"}",
        "vesta-ui-language": "fr",
        __vesta_config_migrated_from_local_storage__: "true",
      },
      recovered_backup_path: null,
    })
    .mockResolvedValueOnce(undefined);
  vi.doMock("$lib/services/tauriClient", () => ({ invokeCommand: mockInvoke }));
  vi.stubGlobal("localStorage", { length: 0, key: () => null, getItem: () => null, clear: vi.fn() });

  try {
    const mod = await import("./vestaConfig");
    await mod.hydrate();

    expect(mod.getItem("vesta-api-keys")).toBe("[{\"id\":\"legacy\"}]");
    expect(mod.getItem("vesta-custom-models")).toBe("[{\"id\":\"custom\"}]");
    expect(mod.getItem("vesta-default-whisper-model")).toBe("small");
    expect(mod.getItem("vesta-ui-language")).toBe("fr");
    expect(mod.getItem("vesta-shortcut-overrides")).toBe("{\"sync-new\":\"Ctrl+K\"}");
    for (const oldKey of Object.keys(mod.LEGACY_KEY_RENAMES)) {
      expect(mod.getItem(oldKey)).toBeNull();
    }
    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenLastCalledWith("config_replace_all", {
      values: {
        "vesta-api-keys": "[{\"id\":\"legacy\"}]",
        "vesta-custom-models": "[{\"id\":\"custom\"}]",
        "vesta-default-whisper-model": "small",
        "vesta-ui-language": "fr",
        "vesta-shortcut-overrides": "{\"sync-new\":\"Ctrl+K\"}",
        __vesta_config_migrated_from_local_storage__: "true",
      },
    });

    // Subsequent startup with already-migrated keys issues no extra write.
    vi.resetModules();
    const cleanInvoke = vi.fn().mockResolvedValueOnce({
      entries: {
        "vesta-api-keys": "[]",
        __vesta_config_migrated_from_local_storage__: "true",
      },
      recovered_backup_path: null,
    });
    vi.doMock("$lib/services/tauriClient", () => ({ invokeCommand: cleanInvoke }));
    const cleanMod = await import("./vestaConfig");
    await cleanMod.hydrate();
    expect(cleanInvoke).toHaveBeenCalledTimes(1);
    expect(cleanInvoke).toHaveBeenCalledWith("config_load_all");
  } finally {
    vi.doUnmock("$lib/services/tauriClient");
    vi.unstubAllGlobals();
  }
});

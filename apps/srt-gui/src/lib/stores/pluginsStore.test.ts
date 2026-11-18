// @vitest-environment happy-dom

import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: any }) => void>();
  return {
    listeners,
    listen: vi.fn(async (eventName: string, handler: (event: { payload: any }) => void) => {
      listeners.set(eventName, handler);
      return () => {
        listeners.delete(eventName);
      };
    }),
    snackbarShow: vi.fn(),
    pluginList: vi.fn(),
    pluginEnable: vi.fn(),
    pluginDisable: vi.fn(),
    pluginUninstall: vi.fn(),
    pluginUpdate: vi.fn(),
    pluginInspect: vi.fn(),
    pluginInstall: vi.fn(),
    pluginSaveSettings: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

vi.mock("$lib/stores/snackbarStore.svelte", () => ({
  snackbar: {
    show: mocks.snackbarShow,
  },
}));

vi.mock("$lib/i18n", async () => {
  const { writable } = await import("svelte/store");
  return {
    locale: writable((key: string, params?: Record<string, string | number>) =>
      params ? `${key}:${JSON.stringify(params)}` : key
    ),
  };
});

vi.mock("$lib/services/pluginClient", () => ({
  pluginList: mocks.pluginList,
  pluginEnable: mocks.pluginEnable,
  pluginDisable: mocks.pluginDisable,
  pluginUninstall: mocks.pluginUninstall,
  pluginUpdate: mocks.pluginUpdate,
  pluginInspect: mocks.pluginInspect,
  pluginInstall: mocks.pluginInstall,
  pluginSaveSettings: mocks.pluginSaveSettings,
}));

import { pluginsStore } from "./pluginsStore.svelte";

const samplePlugin = {
  id: "sub-plugin",
  name: "Sub Plugin",
  version: "1.2.3",
  enabled: true,
  sourceUrl: "https://github.com/example/sub-plugin",
  installedAt: "2026-01-01T00:00:00Z",
  manifest: {
    id: "sub-plugin",
    name: "Sub Plugin",
    version: "1.2.3",
    apiVersion: 1,
    entry: { command: ["python3", "main.py"] },
    permissions: [],
  },
  settings: {},
};

describe("pluginsStore", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    pluginsStore.plugins = [samplePlugin];
    pluginsStore.clearProgress("sub-plugin");
    pluginsStore.clearProgress("other-plugin");
    mocks.pluginList.mockResolvedValue([samplePlugin]);
    if (!pluginsStore.initialized) {
      await pluginsStore.init();
    }
  });

  it("tracks plugin:progress per plugin and id and clears on completion or lifecycle changes", async () => {
    const progressListener = mocks.listeners.get("plugin:progress");
    expect(progressListener).toBeTypeOf("function");

    progressListener!({
      payload: { pluginId: "sub-plugin", id: "download", value: 42, label: "Downloading..." },
    });
    progressListener!({
      payload: { pluginId: "sub-plugin", id: "extract", value: 15, label: "Extracting..." },
    });
    progressListener!({
      payload: { pluginId: "other-plugin", id: "job", value: 70, label: "Other job" },
    });

    expect(pluginsStore.getProgress("sub-plugin")).toEqual([
      { id: "download", value: 42, label: "Downloading..." },
      { id: "extract", value: 15, label: "Extracting..." },
    ]);
    expect(pluginsStore.getProgress("other-plugin")).toEqual([
      { id: "job", value: 70, label: "Other job" },
    ]);

    // Completing one task (>= 100) removes only that entry
    progressListener!({
      payload: { pluginId: "sub-plugin", id: "download", value: 100, label: "Done" },
    });
    expect(pluginsStore.getProgress("sub-plugin")).toEqual([
      { id: "extract", value: 15, label: "Extracting..." },
    ]);

    // Disabling the plugin clears remaining progress for that plugin
    mocks.pluginDisable.mockResolvedValueOnce(undefined);
    mocks.pluginList.mockResolvedValueOnce([
      { ...samplePlugin, enabled: false },
      { ...samplePlugin, id: "other-plugin", enabled: true },
    ]);
    await pluginsStore.disable("sub-plugin");
    expect(pluginsStore.getProgress("sub-plugin")).toEqual([]);
    expect(pluginsStore.getProgress("other-plugin")).toHaveLength(1);

    // Refresh prunes progress for plugins missing or disabled in pluginList
    mocks.pluginList.mockResolvedValueOnce([samplePlugin]);
    await pluginsStore.refresh();
    expect(pluginsStore.getProgress("other-plugin")).toEqual([]);

    // Uninstalling clears active progress
    progressListener!({
      payload: { pluginId: "sub-plugin", id: "op", value: 55, label: "Working" },
    });
    mocks.pluginUninstall.mockResolvedValueOnce(undefined);
    mocks.pluginList.mockResolvedValueOnce([]);
    await pluginsStore.uninstall("sub-plugin");
    expect(pluginsStore.getProgress("sub-plugin")).toEqual([]);
  });

  it("dispatches vesta-plugin-open-flashcards with videoPath and subtitlePath", () => {
    const listener = mocks.listeners.get("plugin:openInFlashcards");
    expect(listener).toBeTypeOf("function");

    const received: any[] = [];
    const onHandoff = (e: Event) => received.push((e as CustomEvent).detail);
    window.addEventListener("vesta-plugin-open-flashcards", onHandoff);
    try {
      listener!({
        payload: { videoPath: "/media/movie.mp4", subtitlePath: "/media/movie.srt" },
      });
      expect(received).toEqual([
        { videoPath: "/media/movie.mp4", subtitlePath: "/media/movie.srt" },
      ]);
    } finally {
      window.removeEventListener("vesta-plugin-open-flashcards", onHandoff);
    }
  });

  it("uses localized i18n keys for all plugin lifecycle snackbar notifications", async () => {
    mocks.pluginInstall.mockResolvedValueOnce(samplePlugin);
    await pluginsStore.install("https://github.com/example/sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.installSuccess:{"name":"Sub Plugin"}',
      "success"
    );

    mocks.pluginInstall.mockRejectedValueOnce(new Error("boom install"));
    await expect(pluginsStore.install("https://github.com/example/bad")).rejects.toThrow("boom install");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.installFailed:{"error":"boom install"}',
      "error"
    );

    mocks.pluginEnable.mockRejectedValueOnce(new Error("boom enable"));
    await pluginsStore.enable("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.enableFailed:{"error":"boom enable"}',
      "error"
    );

    mocks.pluginDisable.mockRejectedValueOnce(new Error("boom disable"));
    await pluginsStore.disable("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.disableFailed:{"error":"boom disable"}',
      "error"
    );

    mocks.pluginUninstall.mockResolvedValueOnce(undefined);
    await pluginsStore.uninstall("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith("plugins.uninstallSuccess", "info");

    mocks.pluginUninstall.mockRejectedValueOnce(new Error("boom uninstall"));
    await pluginsStore.uninstall("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.uninstallFailed:{"error":"boom uninstall"}',
      "error"
    );

    mocks.pluginUpdate.mockResolvedValueOnce({ ...samplePlugin, version: "2.0.0" });
    await pluginsStore.update("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.updateSuccess:{"name":"Sub Plugin","version":"2.0.0"}',
      "success"
    );

    mocks.pluginUpdate.mockRejectedValueOnce(new Error("boom update"));
    await pluginsStore.update("sub-plugin");
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.updateFailed:{"error":"boom update"}',
      "error"
    );

    mocks.pluginSaveSettings.mockResolvedValueOnce(undefined);
    await pluginsStore.saveSettings("sub-plugin", { apiKey: "123" });
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith("plugins.settingsSaved", "success");

    mocks.pluginSaveSettings.mockRejectedValueOnce(new Error("boom settings"));
    await pluginsStore.saveSettings("sub-plugin", { apiKey: "123" });
    expect(mocks.snackbarShow).toHaveBeenLastCalledWith(
      'plugins.settingsSaveFailed:{"error":"boom settings"}',
      "error"
    );
  });
});

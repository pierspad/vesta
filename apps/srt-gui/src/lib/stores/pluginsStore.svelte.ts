import { get } from "svelte/store";
import { locale } from "$lib/i18n";
import {
  pluginList,
  pluginEnable,
  pluginDisable,
  pluginUninstall,
  pluginUpdate,
  pluginInspect,
  pluginInstall,
  pluginSaveSettings,
  type InstalledPlugin,
  type PluginInspectResult,
} from "$lib/services/pluginClient";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { listen } from "@tauri-apps/api/event";

export interface PluginProgressEntry {
  id: string;
  value: number;
  label: string;
}

class PluginsStore {
  plugins = $state<InstalledPlugin[]>([]);
  loading = $state(false);
  initialized = $state(false);
  error = $state<string | null>(null);
  progressByPlugin = $state<Record<string, Record<string, PluginProgressEntry>>>({});

  enabledPlugins = $derived(this.plugins.filter((p) => p.enabled));
  enabledPluginsWithTabs = $derived(
    this.plugins.filter((p) => p.enabled && p.manifest?.tab?.title)
  );

  private listenersSetUp = false;

  async init(): Promise<void> {
    if (this.loading) return;
    this.loading = true;
    this.error = null;

    try {
      this.plugins = await pluginList();
      this.initialized = true;
      this.setupListeners();
    } catch (e: any) {
      this.error = e?.message || String(e);
    } finally {
      this.loading = false;
    }
  }

  private async setupListeners(): Promise<void> {
    if (this.listenersSetUp || typeof window === "undefined") return;
    this.listenersSetUp = true;

    try {
      await listen<{ pluginId: string; level: string; msg: string }>(
        "plugin:notify",
        (event) => {
          const { level, msg } = event.payload;
          const variant =
            level === "error"
              ? "error"
              : level === "warn"
                ? "warning"
                : level === "success"
                  ? "success"
                  : "info";
          snackbar.show(msg, variant);
        }
      );

      await listen<{ pluginId: string; id: string; value: number; label: string }>(
        "plugin:progress",
        (event) => {
          this.updateProgress(event.payload);
        }
      );

      await listen<{ pluginId: string; screen: string; root?: any; patch?: any }>(
        "plugin:update",
        (event) => {
          window.dispatchEvent(
            new CustomEvent("vesta-plugin-ui-update", {
              detail: event.payload,
            })
          );
        }
      );

      await listen<{ videoPath: string; subtitlePath: string }>(
        "plugin:openInFlashcards",
        (event) => {
          window.dispatchEvent(
            new CustomEvent("vesta-plugin-open-flashcards", {
              detail: event.payload,
            })
          );
        }
      );
    } catch {
      // In non-Tauri or test environments, listener registration is safely skipped.
    }
  }

  updateProgress(payload: { pluginId: string; id?: string; value: number; label?: string }): void {
    const pluginId = payload?.pluginId;
    if (!pluginId) return;
    const entryId = payload.id || "default";
    const numericValue = Number(payload.value);
    if (!Number.isFinite(numericValue) || numericValue >= 100) {
      this.clearProgress(pluginId, entryId);
      return;
    }
    const existingPlugin = this.progressByPlugin[pluginId] ?? {};
    this.progressByPlugin = {
      ...this.progressByPlugin,
      [pluginId]: {
        ...existingPlugin,
        [entryId]: {
          id: entryId,
          value: Math.max(0, numericValue),
          label: payload.label ?? "",
        },
      },
    };
  }

  clearProgress(pluginId: string, entryId?: string): void {
    const existingPlugin = this.progressByPlugin[pluginId];
    if (!existingPlugin) return;
    if (entryId !== undefined) {
      if (!(entryId in existingPlugin)) return;
      const nextPlugin = { ...existingPlugin };
      delete nextPlugin[entryId];
      const next = { ...this.progressByPlugin };
      if (Object.keys(nextPlugin).length === 0) {
        delete next[pluginId];
      } else {
        next[pluginId] = nextPlugin;
      }
      this.progressByPlugin = next;
      return;
    }
    const next = { ...this.progressByPlugin };
    delete next[pluginId];
    this.progressByPlugin = next;
  }

  getProgress(pluginId: string): PluginProgressEntry[] {
    const entries = this.progressByPlugin[pluginId];
    return entries ? Object.values(entries) : [];
  }

  async inspect(url: string, gitRef?: string): Promise<PluginInspectResult> {
    return pluginInspect(url, gitRef);
  }

  async install(url: string, gitRef?: string): Promise<InstalledPlugin> {
    this.loading = true;
    this.error = null;
    const t = get(locale);
    try {
      const installed = await pluginInstall(url, gitRef);
      await this.refresh();
      snackbar.show(t("plugins.installSuccess", { name: installed.name }), "success");
      return installed;
    } catch (e: any) {
      const msg = e?.message || String(e);
      this.error = msg;
      snackbar.show(t("plugins.installFailed", { error: msg }), "error");
      throw e;
    } finally {
      this.loading = false;
    }
  }

  async enable(id: string): Promise<void> {
    const t = get(locale);
    try {
      this.clearProgress(id);
      await pluginEnable(id);
      await this.refresh();
    } catch (e: any) {
      snackbar.show(t("plugins.enableFailed", { error: e?.message || String(e) }), "error");
    }
  }

  async disable(id: string): Promise<void> {
    const t = get(locale);
    try {
      await pluginDisable(id);
      this.clearProgress(id);
      await this.refresh();
    } catch (e: any) {
      snackbar.show(t("plugins.disableFailed", { error: e?.message || String(e) }), "error");
    }
  }

  async uninstall(id: string): Promise<void> {
    const t = get(locale);
    try {
      await pluginUninstall(id);
      this.clearProgress(id);
      await this.refresh();
      snackbar.show(t("plugins.uninstallSuccess"), "info");
    } catch (e: any) {
      snackbar.show(t("plugins.uninstallFailed", { error: e?.message || String(e) }), "error");
    }
  }

  async update(id: string, releaseTag?: string): Promise<void> {
    this.loading = true;
    const t = get(locale);
    try {
      const updated = await pluginUpdate(id, releaseTag);
      this.clearProgress(id);
      await this.refresh();
      snackbar.show(t("plugins.updateSuccess", { name: updated.name, version: updated.version }), "success");
    } catch (e: any) {
      snackbar.show(t("plugins.updateFailed", { error: e?.message || String(e) }), "error");
    } finally {
      this.loading = false;
    }
  }

  async saveSettings(id: string, settings: Record<string, any>): Promise<void> {
    const t = get(locale);
    try {
      await pluginSaveSettings(id, settings);
      await this.refresh();
      snackbar.show(t("plugins.settingsSaved"), "success");
    } catch (e: any) {
      snackbar.show(t("plugins.settingsSaveFailed", { error: e?.message || String(e) }), "error");
    }
  }

  async refresh(): Promise<void> {
    try {
      this.plugins = await pluginList();
      const activeIds = new Set(this.plugins.filter((p) => p.enabled).map((p) => p.id));
      for (const pid of Object.keys(this.progressByPlugin)) {
        if (!activeIds.has(pid)) {
          this.clearProgress(pid);
        }
      }
    } catch (e: any) {
      this.error = e?.message || String(e);
    }
  }
}

export const pluginsStore = new PluginsStore();

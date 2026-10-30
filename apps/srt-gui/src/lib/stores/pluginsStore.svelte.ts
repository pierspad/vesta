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

class PluginsStore {
  plugins = $state<InstalledPlugin[]>([]);
  loading = $state(false);
  initialized = $state(false);
  error = $state<string | null>(null);

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

  async inspect(url: string, gitRef?: string): Promise<PluginInspectResult> {
    return pluginInspect(url, gitRef);
  }

  async install(url: string, gitRef?: string): Promise<InstalledPlugin> {
    this.loading = true;
    this.error = null;
    try {
      const installed = await pluginInstall(url, gitRef);
      await this.refresh();
      snackbar.show(`Plugin '${installed.name}' installed successfully`, "success");
      return installed;
    } catch (e: any) {
      const msg = e?.message || String(e);
      this.error = msg;
      snackbar.show(`Install failed: ${msg}`, "error");
      throw e;
    } finally {
      this.loading = false;
    }
  }

  async enable(id: string): Promise<void> {
    try {
      await pluginEnable(id);
      await this.refresh();
    } catch (e: any) {
      snackbar.show(`Failed to enable plugin: ${e?.message || e}`, "error");
    }
  }

  async disable(id: string): Promise<void> {
    try {
      await pluginDisable(id);
      await this.refresh();
    } catch (e: any) {
      snackbar.show(`Failed to disable plugin: ${e?.message || e}`, "error");
    }
  }

  async uninstall(id: string): Promise<void> {
    try {
      await pluginUninstall(id);
      await this.refresh();
      snackbar.show("Plugin uninstalled", "info");
    } catch (e: any) {
      snackbar.show(`Failed to uninstall plugin: ${e?.message || e}`, "error");
    }
  }

  async update(id: string): Promise<void> {
    this.loading = true;
    try {
      const updated = await pluginUpdate(id);
      await this.refresh();
      snackbar.show(`Plugin '${updated.name}' updated to v${updated.version}`, "success");
    } catch (e: any) {
      snackbar.show(`Update failed: ${e?.message || e}`, "error");
    } finally {
      this.loading = false;
    }
  }

  async saveSettings(id: string, settings: Record<string, any>): Promise<void> {
    try {
      await pluginSaveSettings(id, settings);
      await this.refresh();
      snackbar.show("Settings saved", "success");
    } catch (e: any) {
      snackbar.show(`Failed to save settings: ${e?.message || e}`, "error");
    }
  }

  async refresh(): Promise<void> {
    try {
      this.plugins = await pluginList();
    } catch (e: any) {
      this.error = e?.message || String(e);
    }
  }
}

export const pluginsStore = new PluginsStore();

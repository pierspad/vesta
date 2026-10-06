import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { fetch as tauriFetch } from "$lib/services/tauriHttp";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-shell";
import { compareVersions, officialReleaseUrl, selectInstaller, type UpdateAsset } from "$lib/utils/updateVersion";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { t } from "$lib/i18n";
import * as vestaConfig from "$lib/config/vestaConfig";

export type UpdateStatus = "idle" | "checking" | "available" | "current" | "error" | "disabled" | "offline";

const RELEASE_API_URL = "https://api.github.com/repos/pierspad/vesta/releases/latest";
const RELEASES_PAGE_URL = "https://github.com/pierspad/vesta/releases";

/** Self-contained "overview" card: startup auto-check toggle + manual check
 * button. `init()` is called once from SettingsTab.svelte's app-lifetime
 * `onMount` (not from this feature's own panel, which unmounts on section
 * navigation) so the background auto-check keeps firing for users who never
 * open Settings -> Overview -- same reasoning as whisperModelsStore's
 * refreshModels()/refreshAddons() calls, see [[vesta-settings-refactor]]. */
class UpdateCheckerStore {
  automaticUpdateChecks = $state(true);
  installation = $state<{ channel: string; os: string; arch: string } | null>(null);
  installing = $state(false);
  private releaseAssets = $state<UpdateAsset[]>([]);
  downloadPercent = $state(0);
  private initialized = false;
  private checkInFlight = false;

  get canInstall() {
    return !!this.installation && !!selectInstaller(this.releaseAssets, this.installation.channel, this.installation.arch, this.latestVersion);
  }
  get managerHint() {
    switch (this.installation?.channel) {
      case "aur": return t("settings.updatesViaAur");
      case "flatpak": return t("settings.updatesViaFlatpak");
      case "snap": return t("settings.updatesViaSnap");
      default: return "";
    }
  }
  async openRelease() {
    try { await open(officialReleaseUrl(this.releaseUrl) || RELEASES_PAGE_URL); }
    catch (error) { snackbar.show(String(error), "error"); }
  }
  async installUpdate() {
    if (this.installing || !this.canInstall || this.updateStatus !== "available") return;
    this.installing = true;
    this.downloadPercent = 0;
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<number>("update-download-progress", (event) => {
        this.downloadPercent = Math.max(this.downloadPercent, Math.min(100, event.payload));
      });
      await invoke("install_release_update", { version: this.latestVersion });
      snackbar.show(t("settings.updatesInstallerOpened"), "success");
    } catch (error) { snackbar.show(String(error), "error"); }
    finally { unlisten?.(); this.installing = false; }
  }
  updateStatus = $state<UpdateStatus>("idle");
  latestVersion = $state("");
  releaseUrl = $state(RELEASES_PAGE_URL);
  appVersionNum = $state("");
  updateError = $state("");

  private processUpdateResult(source: "auto" | "manual") {
    const comparison = compareVersions(this.latestVersion, this.appVersionNum);
    if (comparison === null) throw new Error("Cannot compare app and release versions");
    this.updateStatus = comparison > 0 ? "available" : "current";
    if (source === "manual") {
      snackbar.show(comparison > 0
        ? t("settings.updatesNewVersionAvailable", { version: this.latestVersion })
        : t("settings.updatesUpToDate"), comparison > 0 ? "info" : "success");
    }
  }

  async checkForUpdates(source: "auto" | "manual" = "manual") {
    if (this.checkInFlight || this.installing) return;
    if (source === "auto" && !this.automaticUpdateChecks) {
      this.updateStatus = "disabled";
      return;
    }

    if (typeof navigator !== "undefined" && navigator.onLine === false) {
      this.updateStatus = "offline";
      if (source === "manual") {
        snackbar.show(t("settings.updatesOffline"), "error");
      }
      return;
    }

    this.checkInFlight = true;
    try {
      this.updateStatus = "checking";
      this.updateError = "";

      const userAgent = "Vesta-update-check";

      // 1. Primary Strategy: GitHub official API via CORS-free tauriFetch
      try {
        const response = await tauriFetch(RELEASE_API_URL, {
          method: "GET",
          headers: { "Accept": "application/vnd.github+json", "User-Agent": userAgent },
        });
        if (!response.ok) throw new Error(`GitHub API returned status ${response.status}`);

        const latest = await response.json() as { tag_name?: string; name?: string; html_url?: string; draft?: boolean; prerelease?: boolean; assets?: UpdateAsset[] };
        const tag = latest.tag_name || latest.name || "";
        if (!tag || latest.draft || latest.prerelease || !officialReleaseUrl(latest.html_url)) throw new Error("Invalid stable release in API response");

        this.latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
        this.releaseUrl = officialReleaseUrl(latest.html_url)!;
        this.releaseAssets = latest.assets || [];
        this.processUpdateResult(source);
        return;
      } catch (apiError) {
        console.warn("Vesta update check: GitHub API failed, trying release redirect:", apiError);
      }

      // 2. Fallback: Redirect check via tauriFetch with redirect: "manual"
      try {
        const response = await tauriFetch(RELEASES_PAGE_URL + "/latest", {
          method: "GET",
          redirect: "manual",
          headers: { "User-Agent": userAgent },
        });

        let tag = "";
        let finalUrl = "";

        const location = response.headers.get("location");
        if ((response.status >= 300 && response.status < 400) && location) {
          finalUrl = location;
          tag = location.substring(location.lastIndexOf("/") + 1);
        } else if (response.ok) {
          finalUrl = response.url || "";
          tag = finalUrl.substring(finalUrl.lastIndexOf("/") + 1);
        }

        if (!tag || !officialReleaseUrl(finalUrl)) throw new Error("Could not parse official release redirect");

        this.latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
        this.releaseUrl = officialReleaseUrl(finalUrl)!;
        this.releaseAssets = [];
        this.processUpdateResult(source);
        return;
      } catch (redirectError) {
        console.error("Vesta update check: All strategies failed:", redirectError);
        this.updateStatus = "error";
        this.updateError = t("settings.updatesCheckFailed");
        if (source === "manual") {
          snackbar.show(this.updateError, "error");
        }
      }
    } finally { this.checkInFlight = false; }
  }

  onAutomaticUpdateChecksChange() {
    vestaConfig.setItem("vesta-automatic-update-checks", this.automaticUpdateChecks.toString());
    if (this.automaticUpdateChecks) {
      void this.checkForUpdates("manual");
    } else if (!this.checkInFlight) {
      this.updateStatus = "disabled";
    }
  }

  /** Called once from SettingsTab.svelte's app-lifetime onMount. */
  init() {
    if (this.initialized) return;
    this.initialized = true;
    void invoke<{ channel: string; os: string; arch: string }>("get_update_installation").then(info => { this.installation = info; }).catch(() => {});
    const savedAutoCheck = vestaConfig.getItem("vesta-automatic-update-checks");
    this.automaticUpdateChecks = savedAutoCheck !== "false";

    invoke<{ version: string }>("get_app_info")
      .then((info) => {
        this.appVersionNum = `v${info.version}`;
      })
      .catch(() => {
        this.appVersionNum = "";
      })
      .finally(() => {
        if (this.automaticUpdateChecks) {
          void this.checkForUpdates("auto");
        } else {
          this.updateStatus = "disabled";
        }
      });
  }
}

export const updateCheckerStore = new UpdateCheckerStore();

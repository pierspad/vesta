import { get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { fetch as tauriFetch } from "./tauriHttp";
import { snackbar } from "./snackbarStore.svelte";
import { currentLanguage } from "./i18n";

export type UpdateStatus = "idle" | "checking" | "available" | "current" | "error" | "disabled" | "offline";

const RELEASE_API_URL = "https://api.github.com/repos/pierspad/vesta/releases/latest";
const RELEASES_PAGE_URL = "https://github.com/pierspad/vesta/releases";

function normalizeVersion(version: string): string {
  return version.trim().replace(/^v/i, "").split(/[+-]/)[0];
}

function compareVersions(left: string, right: string): number {
  const leftParts = normalizeVersion(left).split(".").map((part) => Number.parseInt(part, 10) || 0);
  const rightParts = normalizeVersion(right).split(".").map((part) => Number.parseInt(part, 10) || 0);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let i = 0; i < length; i += 1) {
    const diff = (leftParts[i] || 0) - (rightParts[i] || 0);
    if (diff !== 0) return diff;
  }

  return 0;
}

/** Self-contained "overview" card: startup auto-check toggle + manual check
 * button. `init()` is called once from SettingsTab.svelte's app-lifetime
 * `onMount` (not from this feature's own panel, which unmounts on section
 * navigation) so the background auto-check keeps firing for users who never
 * open Settings -> Overview -- same reasoning as whisperModelsStore's
 * refreshModels()/refreshAddons() calls, see [[vesta-settings-refactor]]. */
class UpdateCheckerStore {
  automaticUpdateChecks = $state(true);
  updateStatus = $state<UpdateStatus>("idle");
  latestVersion = $state("");
  releaseUrl = $state(RELEASES_PAGE_URL);
  appVersionNum = $state("");
  updateError = $state("");

  private processUpdateResult(source: "auto" | "manual") {
    const it = get(currentLanguage) === "it";
    if (this.appVersionNum) {
      if (compareVersions(this.latestVersion, this.appVersionNum) > 0) {
        this.updateStatus = "available";
        if (source === "manual") {
          snackbar.show((it ? "Nuova versione disponibile: " : "New version available: ") + this.latestVersion, "info");
        }
      } else {
        this.updateStatus = "current";
        if (source === "manual") {
          snackbar.show(it ? "Il software è aggiornato" : "Software is up to date", "success");
        }
      }
    } else {
      this.updateStatus = "current";
    }
  }

  async checkForUpdates(source: "auto" | "manual" = "manual") {
    if (source === "auto" && !this.automaticUpdateChecks) {
      this.updateStatus = "disabled";
      return;
    }

    const it = get(currentLanguage) === "it";

    if (typeof navigator !== "undefined" && navigator.onLine === false) {
      this.updateStatus = "offline";
      if (source === "manual") {
        snackbar.show(it ? "Connessione assente o GitHub non raggiungibile" : "No connection or GitHub is unreachable", "error");
      }
      return;
    }

    this.updateStatus = "checking";
    this.updateError = "";

    const userAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

    // 1. Primary Strategy: GitHub official API via CORS-free tauriFetch
    try {
      const response = await tauriFetch(RELEASE_API_URL, {
        method: "GET",
        headers: { "Accept": "application/vnd.github+json", "User-Agent": userAgent },
      });
      if (!response.ok) throw new Error(`GitHub API returned status ${response.status}`);

      const latest = await response.json() as { tag_name?: string; name?: string; html_url?: string };
      const tag = latest.tag_name || latest.name || "";
      if (!tag) throw new Error("Empty version tag in API response");

      this.latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      this.releaseUrl = latest.html_url || RELEASES_PAGE_URL;
      this.processUpdateResult(source);
      return;
    } catch (apiError) {
      console.warn("Vesta update check: GitHub API failed, trying package.json fallback:", apiError);
    }

    // 2. Secondary Strategy: Raw package.json via CORS-free tauriFetch (rate-limit free!)
    try {
      const response = await tauriFetch("https://raw.githubusercontent.com/pierspad/vesta/main/apps/srt-gui/package.json", {
        method: "GET",
        headers: { "User-Agent": userAgent },
      });
      if (!response.ok) throw new Error(`Raw package.json fetch returned status ${response.status}`);

      const pkg = await response.json() as { version?: string };
      const tag = pkg.version || "";
      if (!tag) throw new Error("Empty version field in package.json");

      this.latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      this.releaseUrl = RELEASES_PAGE_URL;
      this.processUpdateResult(source);
      return;
    } catch (pkgError) {
      console.warn("Vesta update check: Raw package.json fallback failed, trying redirect fallback:", pkgError);
    }

    // 3. Tertiary Strategy: Redirect check via tauriFetch with redirect: "manual"
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

      if (!tag || tag === "latest") throw new Error("Could not parse redirect version tag");

      this.latestVersion = tag.startsWith("v") || tag.startsWith("V") ? tag : `v${tag}`;
      this.releaseUrl = finalUrl || RELEASES_PAGE_URL;
      this.processUpdateResult(source);
      return;
    } catch (redirectError) {
      console.error("Vesta update check: All strategies failed:", redirectError);
      this.updateStatus = "error";
      this.updateError = get(currentLanguage) === "it" ? "Impossibile controllare gli aggiornamenti" : "Could not check for updates";
      if (source === "manual") {
        snackbar.show(this.updateError, "error");
      }
    }
  }

  onAutomaticUpdateChecksChange() {
    localStorage.setItem("vesta-automatic-update-checks", this.automaticUpdateChecks.toString());
    if (this.automaticUpdateChecks) {
      void this.checkForUpdates("manual");
    } else {
      this.updateStatus = "disabled";
    }
  }

  /** Called once from SettingsTab.svelte's app-lifetime onMount. */
  init() {
    const savedAutoCheck = localStorage.getItem("vesta-automatic-update-checks");
    this.automaticUpdateChecks = savedAutoCheck !== "false";

    invoke<{ version: string }>("get_app_info")
      .then((info) => {
        this.appVersionNum = `v${info.version}`;
      })
      .catch(() => {
        this.appVersionNum = "v0.14.1-dev.2";
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

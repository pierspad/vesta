<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { locale } from "./i18n";

  const RELEASE_API_URL = "https://api.github.com/repos/pierspad/Vesta/releases/latest";
  const RELEASES_URL = "https://github.com/pierspad/Vesta/releases";
  type ReleaseStatus = "idle" | "checking" | "available" | "current" | "offline";

  let t = $derived($locale);
  let releaseStatus = $state<ReleaseStatus>("idle");
  let appVersionNum = $state("");
  let latestVersion = $state("");
  let releaseUrl = $state(RELEASES_URL);
  let releaseCheckedAt = $state<Date | null>(null);

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

  function formatVersionTag(version: string): string {
    const normalized = version.trim();
    if (!normalized) return "";
    return normalized.startsWith("v") || normalized.startsWith("V") ? normalized : `v${normalized}`;
  }

  function formatCheckedAt(value: Date | null): string {
    if (!value) return "";
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "short",
      timeStyle: "short",
    }).format(value);
  }

  async function checkForUpdates(currentVersion: string) {
    if (!currentVersion) return;
    if (typeof navigator !== "undefined" && navigator.onLine === false) {
      releaseStatus = "offline";
      return;
    }

    releaseStatus = "checking";

    try {
      const response = await fetch(RELEASE_API_URL, {
        cache: "no-store",
        headers: { Accept: "application/vnd.github+json" },
      });

      if (!response.ok) throw new Error(`GitHub release check failed: ${response.status}`);

      const latest = await response.json() as {
        tag_name?: string;
        name?: string;
        html_url?: string;
      };
      const tag = latest.tag_name || latest.name || "";
      latestVersion = formatVersionTag(tag);
      releaseUrl = latest.html_url || RELEASES_URL;
      releaseCheckedAt = new Date();
      releaseStatus = compareVersions(latestVersion, currentVersion) > 0 ? "available" : "current";
    } catch (error) {
      console.warn("Could not check Vesta releases:", error);
      releaseStatus = "offline";
    }
  }

  onMount(() => {
    invoke<{ version: string; name: string; license: string }>("get_app_info")
      .then((info) => {
        appVersionNum = `v${info.version}`;
        void checkForUpdates(info.version);
      })
      .catch(() => {
        appVersionNum = "v0.1.0";
        releaseStatus = "offline";
      });
  });
</script>

<div class="h-full flex flex-col p-6 overflow-y-auto bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950">
  <div class="mb-6 flex items-start justify-between gap-4">
    <div>
      <p class="text-xs uppercase tracking-wide text-gray-500 mb-2">{t("notifications.title")}</p>
      <h2 class="text-2xl font-bold text-white">{t("notifications.centerTitle")}</h2>
      <p class="text-sm text-gray-500 mt-1">{t("notifications.centerDesc")}</p>
    </div>
    <button
      type="button"
      onclick={() => checkForUpdates(normalizeVersion(appVersionNum))}
      class="h-10 px-4 rounded-lg border border-white/10 bg-white/5 text-sm font-semibold text-gray-300 hover:text-white hover:bg-white/10 transition-colors"
      disabled={releaseStatus === "checking"}
    >
      {t("notifications.refresh")}
    </button>
  </div>

  <div class="glass-card p-5 max-w-3xl">
    {#if releaseStatus === "available"}
      <div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4">
        <p class="text-sm font-semibold text-amber-100">
          {t("notifications.updateAvailable", { version: latestVersion })}
        </p>
        <p class="text-xs text-gray-400 mt-1">
          {t("notifications.updateDesc", { current: appVersionNum, latest: latestVersion })}
        </p>
        <a
          href={releaseUrl}
          target="_blank"
          rel="noreferrer"
          class="mt-4 inline-flex items-center gap-2 rounded-lg border border-amber-400/30 bg-amber-400/10 px-3 py-2 text-xs font-semibold text-amber-100 hover:bg-amber-400/15"
        >
          {t("notifications.openReleases")}
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 3h7m0 0v7m0-7L10 14M5 7v14h14v-5" />
          </svg>
        </a>
      </div>
    {:else if releaseStatus === "checking"}
      <p class="text-sm text-gray-400">{t("notifications.checking")}</p>
    {:else if releaseStatus === "offline"}
      <p class="text-sm text-gray-400">{t("notifications.offline")}</p>
    {:else}
      <p class="text-sm text-gray-400">{t("notifications.none")}</p>
    {/if}

    {#if releaseCheckedAt}
      <p class="text-xs text-gray-600 mt-4">
        {t("notifications.lastChecked")}: {formatCheckedAt(releaseCheckedAt)}
      </p>
    {/if}
  </div>
</div>

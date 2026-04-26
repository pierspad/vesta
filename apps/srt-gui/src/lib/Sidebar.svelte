<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import fireplaceIcon from "../assets/fireplace.svg";
  import { locale } from "./i18n";

  interface Props {
    activeTab: "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "notifications" | "shortcuts";
    onTabChange: (tab: "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "notifications" | "shortcuts") => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
  }

  let { activeTab, onTabChange, collapsed = false, onToggleCollapse }: Props = $props();
  
  let t = $derived($locale);

  let appVersion = $state<string>("");

  let appVersionNum = $state("");
  let appLicense = $state("");
  const RELEASE_API_URL = "https://api.github.com/repos/pierspad/Vesta/releases/latest";
  const RELEASES_URL = "https://github.com/pierspad/Vesta/releases";
  type ReleaseStatus = "idle" | "checking" | "available" | "current" | "offline";
  let releaseStatus = $state<ReleaseStatus>("idle");
  let latestVersion = $state("");
  let releaseUrl = $state(RELEASES_URL);
  let releaseCheckedAt = $state<Date | null>(null);
  let hasUpdateNotification = $derived(releaseStatus === "available");

  function formatLicense(license: string): string {
    return license.replace(/-only$/i, "").trim();
  }

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
        appLicense = formatLicense(info.license);
        appVersion = `v${info.version} • Tauri + Svelte • ${formatLicense(info.license)}`;
        void checkForUpdates(info.version);
      })
      .catch(() => {
        appVersion = "VESTA";
        appVersionNum = "v0.1.0";
        appLicense = "GPL-3.0";
      });
  });
</script>

<aside class="{collapsed ? 'w-20' : 'w-72'} bg-gradient-to-b from-gray-900 via-gray-900 to-gray-950 border-r border-white/10 flex flex-col transition-[width] duration-200 ease-out relative will-change-[width]">
  <button
    onclick={onToggleCollapse}
    class="absolute -right-3 top-6 w-6 h-6 bg-gray-800 border border-white/20 rounded-full flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 transition-all z-10 shadow-lg"
    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
  >
    <svg class="w-3 h-3 transition-transform {collapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
    </svg>
  </button>

  <div class="p-6 border-b border-white/10">
    <div class="flex items-center gap-3 {collapsed ? 'justify-center' : ''}">
      <div class="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 relative overflow-visible">
        <div class="absolute -inset-2 bg-orange-500/35 rounded-full blur-xl z-0"></div>
        <div class="absolute inset-0 bg-amber-300/20 rounded-full blur-md z-0"></div>
        <img src={fireplaceIcon} alt="VESTA" class="w-10 h-10 drop-shadow-[0_0_18px_rgba(249,115,22,0.9)] relative z-10" />
      </div>
      {#if !collapsed}
        <div class="relative z-10">
          <h1 class="text-2xl font-bold tracking-wider bg-gradient-to-r from-amber-400 via-orange-400 to-red-400 bg-clip-text text-transparent">
            {t("app.title")}
          </h1>
        </div>
      {/if}
    </div>
  </div>

  <nav class="flex-1 p-4 space-y-2 flex flex-col">
    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'flashcards'
        ? 'bg-gradient-to-r from-emerald-600 to-teal-600 text-white shadow-lg shadow-emerald-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("flashcards")}
      title={collapsed ? t("nav.flashcards") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'flashcards' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left flex-1">
          <span class="block font-medium {activeTab === 'flashcards' ? 'text-white' : ''}">{t("nav.flashcards")}</span>
          <span class="text-xs {activeTab === 'flashcards' ? 'text-white/70' : 'text-gray-500'}">{t("nav.flashcards.desc")}</span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'translate'
        ? 'bg-gradient-to-r from-indigo-600 to-purple-600 text-white shadow-lg shadow-indigo-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("translate")}
      title={collapsed ? t("nav.translate") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'translate' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left">
          <span class="block font-medium">{t("nav.translate")}</span>
          <span class="text-xs {activeTab === 'translate' ? 'text-white/70' : 'text-gray-500'}">{t("nav.translate.desc")}</span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'sync'
        ? 'bg-gradient-to-r from-indigo-600 to-purple-600 text-white shadow-lg shadow-indigo-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("sync")}
      title={collapsed ? t("nav.sync") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'sync' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left flex-1">
          <span class="block font-medium">{t("nav.sync")}</span>
          <span class="text-xs {activeTab === 'sync' ? 'text-white/70' : 'text-gray-500'}">{t("nav.sync.desc")}</span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'align'
        ? 'bg-gradient-to-r from-teal-500 to-emerald-600 text-white shadow-lg shadow-teal-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("align")}
      title={collapsed ? t("nav.revision") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'align' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7"/>
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left flex-1">
          <span class="block font-medium">{t("nav.revision")}</span>
          <span class="text-xs {activeTab === 'align' ? 'text-white/70' : 'text-gray-500'}">{t("nav.revision.desc")}</span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'transcribe'
        ? 'bg-gradient-to-r from-cyan-600 to-blue-600 text-white shadow-lg shadow-cyan-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("transcribe")}
      title={collapsed ? t("nav.transcribe") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'transcribe' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left flex-1">
          <span class="block font-medium {activeTab === 'transcribe' ? 'text-white' : ''}">{t("nav.transcribe")}</span>
          <span class="text-xs {activeTab === 'transcribe' ? 'text-white/70' : 'text-gray-500'}">{t("nav.transcribe.desc")}</span>
        </div>
      {/if}
    </button>

    <div class="flex-1"></div>

    <div class="border-t border-white/10 my-2"></div>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'settings'
        ? 'bg-gradient-to-r from-indigo-600 to-purple-600 text-white shadow-lg shadow-indigo-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("settings")}
      title={collapsed ? t("nav.settings") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'settings' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
          />
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left">
          <span class="block font-medium">{t("nav.settings")}</span>
          <span class="text-xs {activeTab === 'settings' ? 'text-white/70' : 'text-gray-500'}">{t("nav.settings.desc")}</span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab === 'notifications'
        ? 'bg-gradient-to-r from-slate-700 to-slate-600 text-white shadow-lg shadow-slate-500/20'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("notifications")}
      title={collapsed ? t("nav.notifications") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'notifications' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6 6 0 10-12 0v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0a3 3 0 11-6 0m6 0H9"
          />
        </svg>
        {#if hasUpdateNotification}
          <span class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-amber-400 ring-2 ring-gray-900"></span>
        {/if}
      </div>
      {#if !collapsed}
        <div class="text-left flex-1 min-w-0">
          <span class="block font-medium">{t("nav.notifications")}</span>
          <span class="text-xs {activeTab === 'notifications' ? 'text-white/70' : 'text-gray-500'} truncate">
            {#if releaseStatus === "available"}
              {t("nav.notifications.update")}
            {:else if releaseStatus === "checking"}
              {t("notifications.checking")}
            {:else}
              {t("nav.notifications.desc")}
            {/if}
          </span>
        </div>
      {/if}
    </button>

    <button
      class="w-full flex items-center gap-3 {collapsed ? 'px-2 justify-center' : 'px-4'} py-3 rounded-xl transition-all duration-300 {activeTab ===
      'shortcuts'
        ? 'bg-gradient-to-r from-indigo-600 to-purple-600 text-white shadow-lg shadow-indigo-500/30'
        : 'text-gray-400 hover:bg-white/5 hover:text-white'}"
      onclick={() => onTabChange("shortcuts")}
      title={collapsed ? t("nav.shortcuts") : undefined}
    >
      <div class="w-8 h-8 rounded-lg {activeTab === 'shortcuts' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0">
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 7a3 3 0 013-3h10a3 3 0 013 3v10a3 3 0 01-3 3H7a3 3 0 01-3-3V7zm4 2h2m2 0h2m2 0h2M7 13h2m2 0h2m2 0h2M7 17h6"
          />
        </svg>
      </div>
      {#if !collapsed}
        <div class="text-left">
          <span class="block font-medium">{t("nav.shortcuts")}</span>
          <span class="text-xs {activeTab === 'shortcuts' ? 'text-white/70' : 'text-gray-500'}">{t("nav.shortcuts.desc")}</span>
        </div>
      {/if}
    </button>
  </nav>

  <div class="p-4 border-t border-white/10">
    <div class="glass-card p-3 {collapsed ? 'flex items-center justify-center' : ''}">
      {#if collapsed}
        <span class="text-[10px] text-gray-500">
          <a href="https://github.com/pierspad/VESTA.git" target="_blank" class="hover:text-gray-400 transition-colors">
            {appVersionNum || "v0.1"}
          </a>
        </span>
      {:else}
        <div class="w-fit">
          <p class="text-xs text-gray-500">
            <a href="https://github.com/pierspad/VESTA.git" target="_blank" class="hover:text-indigo-400 transition-colors">
              {appVersionNum || "v0.1.0"}
            </a>
            <span class="mx-1">•</span>
            Tauri + Svelte
            <span class="mx-1">•</span>
            <a href="https://www.gnu.org/licenses/gpl-3.0.html" target="_blank" class="hover:text-indigo-400 transition-colors">
              {appLicense || "GPL-3.0"}
            </a>
          </p>
          <p class="text-[10px] text-gray-600 mt-1 flex justify-between items-center">
            <a href="https://pierspad.com" target="_blank" class="hover:text-gray-400 transition-colors">Pierpaolo Spadafora</a>
            <a href="https://github.com/pierspad/VESTA.git" target="_blank" class="hover:text-gray-400 transition-colors flex items-center gap-1">
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24"><path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd" /></svg>
              GitHub Repo
            </a>
          </p>
        </div>
      {/if}
    </div>
  </div>
</aside>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
  import fireplaceIcon from "../assets/fireplace.svg";
  import authorAvatar from "../assets/avatar.png";
  import { locale } from "./i18n";
  import { aiStore } from "./aiStore.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import {
    getStoredSettingsActionState,
    SETTINGS_ACTION_REQUIRED_EVENT,
    type SettingsActionNotificationDetail,
  } from "./settingsNotifications";

  interface Props {
    activeTab: "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "refine";
    onTabChange: (tab: "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "refine") => void;
    collapsed?: boolean;
    onToggleCollapse?: () => void;
    settingsSection?: "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts";
    lastActiveMainTab?: "translate" | "sync" | "transcribe" | "align" | "flashcards" | "refine";
  }

  let { activeTab, onTabChange, collapsed = false, onToggleCollapse, settingsSection = $bindable("overview"), lastActiveMainTab = "flashcards" }: Props = $props();
  
  let t = $derived($locale);

  let appVersion = $state<string>("");

  let appVersionNum = $state("");
  let appLicense = $state("");
  const RELEASE_API_URL = "https://api.github.com/repos/pierspad/Vesta/releases/latest";
  const RELEASES_URL = "https://github.com/pierspad/Vesta/releases";
  
  const repoUrl = "https://github.com/pierspad/VESTA";
  let releasesUrl = $derived(
    appVersionNum
      ? `https://github.com/pierspad/VESTA/releases/tag/${appVersionNum}`
      : "https://github.com/pierspad/VESTA/releases"
  );
  const licenseUrl = "https://github.com/pierspad/VESTA/blob/main/LICENSE";
  const authorUrl = "https://pierspad.com";
  const authorIconUrl = authorAvatar;
  type ReleaseStatus = "idle" | "checking" | "available" | "current" | "offline";
  let releaseStatus = $state<ReleaseStatus>("idle");
  let latestVersion = $state("");
  let releaseUrl = $state(RELEASES_URL);
  let releaseCheckedAt = $state<Date | null>(null);
  let hasUpdateNotification = $derived(releaseStatus === "available");
  let hasSettingsActionNotification = $state(false);
  let settingsNotificationRead = $state(false);
  let settingsHash = $state("");
  let needsWhisperDot = $derived(settingsHash.includes("whisper-model-missing:v1"));
  let needsLlmDot = $derived(settingsHash.includes("llm-default-unready:v1"));

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

    // 1. Primary Strategy: GitHub official API via CORS-free tauriFetch
    try {
      const response = await tauriFetch(RELEASE_API_URL, {
        method: "GET",
        headers: {
          "Accept": "application/vnd.github+json",
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
      });

      if (!response.ok) {
        throw new Error(`GitHub API returned status ${response.status}`);
      }

      const latest = await response.json() as {
        tag_name?: string;
        name?: string;
        html_url?: string;
      };
      
      const tag = latest.tag_name || latest.name || "";
      if (!tag) {
        throw new Error("Empty version tag in API response");
      }

      latestVersion = formatVersionTag(tag);
      releaseUrl = latest.html_url || RELEASES_URL;
      releaseCheckedAt = new Date();
      releaseStatus = compareVersions(latestVersion, currentVersion) > 0 ? "available" : "current";
      return;
    } catch (apiError) {
      console.warn("Vesta background update check: GitHub API failed, trying package.json fallback:", apiError);
    }

    // 2. Secondary Strategy: Raw package.json via CORS-free tauriFetch (rate-limit free!)
    try {
      const response = await tauriFetch("https://raw.githubusercontent.com/pierspad/Vesta/main/apps/srt-gui/package.json", {
        method: "GET",
        headers: {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
      });

      if (!response.ok) {
        throw new Error(`Raw package.json fetch returned status ${response.status}`);
      }

      const pkg = await response.json() as { version?: string };
      const tag = pkg.version || "";
      if (!tag) {
        throw new Error("Empty version field in package.json");
      }

      latestVersion = formatVersionTag(tag);
      releaseUrl = RELEASES_URL;
      releaseCheckedAt = new Date();
      releaseStatus = compareVersions(latestVersion, currentVersion) > 0 ? "available" : "current";
      return;
    } catch (pkgError) {
      console.warn("Vesta background update check: Raw package.json fallback failed, trying redirect fallback:", pkgError);
    }

    // 3. Tertiary Strategy: Redirect check via tauriFetch with redirect: "manual"
    try {
      const response = await tauriFetch("https://github.com/pierspad/Vesta/releases/latest", {
        method: "GET",
        redirect: "manual",
        headers: {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        }
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

      if (!tag || tag === "latest") {
        throw new Error("Could not parse redirect version tag");
      }

      latestVersion = formatVersionTag(tag);
      releaseUrl = finalUrl || RELEASES_URL;
      releaseCheckedAt = new Date();
      releaseStatus = compareVersions(latestVersion, currentVersion) > 0 ? "available" : "current";
      return;
    } catch (redirectError) {
      console.error("Vesta background update check: All strategies failed:", redirectError);
      releaseStatus = "offline";
    }
  }

  $effect(() => {
    if (!uiMode.expertMode) {
      if (settingsSection === "language" || settingsSection === "anki") {
        settingsSection = "overview";
      }
    }
  });

  onMount(() => {
    const initial = getStoredSettingsActionState();
    hasSettingsActionNotification = initial.required;
    settingsNotificationRead = initial.read;
    settingsHash = initial.hash || "";

    const handleSettingsActionRequired = (event: Event) => {
      const detail = (event as CustomEvent<SettingsActionNotificationDetail>).detail;
      if (typeof detail === "boolean") {
        hasSettingsActionNotification = detail;
        return;
      }

      hasSettingsActionNotification = detail.required;
      settingsNotificationRead = detail.read;
      settingsHash = detail.hash || "";
    };

    window.addEventListener(SETTINGS_ACTION_REQUIRED_EVENT, handleSettingsActionRequired);

    const savedAutoCheck = localStorage.getItem("vesta-automatic-update-checks");
    const autoCheckEnabled = savedAutoCheck !== "false";

    invoke<{ version: string; name: string; license: string }>("get_app_info")
      .then((info) => {
        appVersionNum = `v${info.version}`;
        appLicense = formatLicense(info.license);
        appVersion = `v${info.version} • Tauri + Svelte • ${formatLicense(info.license)}`;
        if (autoCheckEnabled) {
          void checkForUpdates(info.version);
        } else {
          releaseStatus = "offline";
        }
      })
      .catch(() => {
        appVersion = "VESTA";
        appVersionNum = "v0.14.0-dev.1";
        appLicense = "GPL-3.0";
      });

    return () => {
      window.removeEventListener(SETTINGS_ACTION_REQUIRED_EVENT, handleSettingsActionRequired);
    };
  });
</script>

<aside class="{collapsed ? 'w-20' : 'w-[260px]'} bg-gray-900 border-r border-white/10 flex flex-col transition-[width] duration-200 ease-out relative will-change-[width]">
  <button
    onclick={onToggleCollapse}
    class="absolute -right-3 top-6 w-6 h-6 bg-gray-800 border border-white/20 rounded-full flex items-center justify-center text-gray-400 hover:text-white hover:bg-gray-700 transition-all z-10 shadow-lg"
    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
  >
    <svg class="w-3 h-3 transition-transform {collapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
    </svg>
  </button>

  <div class="h-[89px] min-h-[89px] max-h-[89px] border-b border-white/10 flex items-center shrink-0 {collapsed ? 'justify-center px-2' : 'pl-[32px] pr-4'}">
    {#if activeTab === "settings"}
      <button
        onclick={() => {
          onTabChange(lastActiveMainTab);
        }}
        class="flex items-center gap-4 group cursor-pointer focus:outline-none {collapsed ? 'w-8 h-8 justify-center' : 'w-full text-left'}"
      >
        <div class="w-8 h-8 rounded-lg bg-white/5 flex items-center justify-center flex-shrink-0 group-hover:bg-white/10 transition-colors border border-white/10">
          <svg class="w-4.5 h-4.5 text-gray-400 group-hover:text-white transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M10.5 19.5 3 12m0 0 7.5-7.5M3 12h18" />
          </svg>
        </div>
        {#if !collapsed}
          <span class="text-lg font-bold text-gray-300 group-hover:text-white transition-colors ml-1">{t("nav.settings")}</span>
        {/if}
      </button>
    {:else}
      <div class="flex items-center gap-3 w-full {collapsed ? 'justify-center' : ''}">
        <button
          onclick={() => onTabChange("settings")}
          class="brand-settings-toggle-btn text-gray-400 hover:text-white transition-colors duration-200 focus:outline-none flex items-center justify-center w-8 h-8 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 relative shrink-0 cursor-pointer"
          title={t("nav.settings")}
        >
          <svg class="w-4.5 h-4.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          {#if hasSettingsActionNotification || hasUpdateNotification}
            {#if hasSettingsActionNotification && !settingsNotificationRead}
              <span class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-red-500 ring-2 ring-gray-900 shadow-[0_0_10px_rgba(239,68,68,0.75)]"></span>
            {:else if hasUpdateNotification}
              <span class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-amber-500 ring-2 ring-gray-900 shadow-[0_0_10px_rgba(245,158,11,0.75)] animate-pulse"></span>
            {:else}
              <span class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-gray-500 ring-2 ring-gray-900"></span>
            {/if}
          {/if}
        </button>

        {#if !collapsed}
          <div class="w-14 h-14 rounded-xl flex items-center justify-center flex-shrink-0 relative overflow-visible ml-2" style="transform: translate3d(0, 0, 0);">
            <div class="absolute -inset-2 bg-orange-500/22 rounded-full blur-lg z-0" style="will-change: filter; transform: translate3d(0, 0, 0);"></div>
            <div class="absolute inset-0 bg-amber-300/12 rounded-full blur-md z-0" style="will-change: filter; transform: translate3d(0, 0, 0);"></div>
            <img src={fireplaceIcon} alt="VESTA" class="w-14 h-14 drop-shadow-[0_0_10px_rgba(249,115,22,0.55)] relative z-10" style="will-change: filter; transform: translate3d(0, 0, 0);" />
          </div>
          <div class="relative z-10">
            <h1 class="text-2xl font-bold tracking-wider bg-gradient-to-r from-amber-400 via-orange-400 to-red-400 bg-clip-text text-transparent">
              {t("app.title")}
            </h1>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <nav class="flex-1 p-4 flex flex-col justify-start gap-3">

    {#if activeTab === "settings"}
      <!-- Settings navigation buttons -->
      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'overview'
          ? 'bg-gradient-to-r from-sky-600 to-sky-700 text-white shadow-lg shadow-sky-500/22 border-sky-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-sky-500/10 hover:text-sky-400 hover:border-sky-500/20'}"
        onclick={() => settingsSection = "overview"}
        title={collapsed ? t("settings.section.overview") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {settingsSection === 'overview' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
          </svg>
        </div>
        {#if !collapsed}
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold">{t("settings.section.overview")}</span>
          </div>
        {/if}
      </button>

      {#if !aiStore.killSwitchActive}
        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'llm'
            ? 'bg-gradient-to-r from-indigo-600 to-indigo-700 text-white shadow-lg shadow-indigo-500/22 border-indigo-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-indigo-500/10 hover:text-indigo-400 hover:border-indigo-500/20'}"
          onclick={() => settingsSection = "llm"}
          title={collapsed ? t("settings.section.llm") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {settingsSection === 'llm' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3h6m-7 4h8a3 3 0 013 3v7a3 3 0 01-3 3H8a3 3 0 01-3-3v-7a3 3 0 01-3-3zm4 3v4m-2-2h4" />
            </svg>
            {#if needsLlmDot}
              <span class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-amber-500 ring-2 ring-gray-900 animate-pulse shadow-[0_0_8px_rgba(245,158,11,0.6)]"></span>
            {/if}
          </div>
          {#if !collapsed}
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold">{t("settings.section.llm")}</span>
            </div>
          {/if}
        </button>

        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'whisper'
            ? 'bg-gradient-to-r from-cyan-600 to-cyan-700 text-white shadow-lg shadow-cyan-500/22 border-cyan-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-cyan-500/10 hover:text-cyan-400 hover:border-cyan-500/20'}"
          onclick={() => settingsSection = "whisper"}
          title={collapsed ? t("settings.section.whisper") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {settingsSection === 'whisper' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 18a6 6 0 006-6V7a6 6 0 10-12 0v5a6 6 0 006 6zm0 0v3m-4 0h8" />
            </svg>
            {#if needsWhisperDot}
              <span class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-amber-500 ring-2 ring-gray-900 animate-pulse shadow-[0_0_8px_rgba(245,158,11,0.6)]"></span>
            {/if}
          </div>
          {#if !collapsed}
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold">{t("settings.section.whisper")}</span>
            </div>
          {/if}
        </button>
      {/if}

      {#if uiMode.expertMode}
        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'language'
            ? 'bg-gradient-to-r from-emerald-600 to-emerald-700 text-white shadow-lg shadow-emerald-500/22 border-emerald-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-emerald-500/10 hover:text-emerald-400 hover:border-emerald-500/20'}"
          onclick={() => settingsSection = "language"}
          title={collapsed ? t("settings.section.language") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {settingsSection === 'language' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1 9a18 18 0 01-4-5m7 12l5-10 5 10m-9-4h8" />
            </svg>
          </div>
          {#if !collapsed}
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold">{t("settings.section.language")}</span>
            </div>
          {/if}
        </button>

        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'anki'
            ? 'bg-gradient-to-r from-amber-600 to-amber-700 text-white shadow-lg shadow-amber-500/22 border-amber-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-amber-500/10 hover:text-amber-400 hover:border-amber-500/20'}"
          onclick={() => settingsSection = "anki"}
          title={collapsed ? t("settings.section.anki") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {settingsSection === 'anki' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v4H4V5zm0 8h8v7H5a1 1 0 01-1-1v-6zm12 0h4v6a1 1 0 01-1 1h-3v-7z" />
            </svg>
          </div>
          {#if !collapsed}
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold">{t("settings.section.anki")}</span>
            </div>
          {/if}
        </button>
      {/if}

      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {settingsSection === 'shortcuts'
          ? 'bg-gradient-to-r from-purple-600 to-purple-700 text-white shadow-lg shadow-purple-500/22 border-purple-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-purple-500/10 hover:text-purple-400 hover:border-purple-500/20'}"
        onclick={() => settingsSection = "shortcuts"}
        title={collapsed ? t("settings.section.shortcuts") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {settingsSection === 'shortcuts' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7a3 3 0 013-3h10a3 3 0 013 3v10a3 3 0 01-3 3H7a3 3 0 01-3-3V7zm4 2h2m2 0h2m2 0h2M7 13h2m2 0h2m2 0h2M7 17h6" />
          </svg>
          {#if hasUpdateNotification}
            <span class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-amber-500 ring-2 ring-gray-900 animate-pulse shadow-[0_0_8px_rgba(245,158,11,0.6)]"></span>
          {/if}
        </div>
        {#if !collapsed}
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold">{t("settings.section.shortcuts")}</span>
          </div>
        {/if}
      </button>
    {:else}
      <!-- 1. Flashcards -->
      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
        'flashcards'
          ? 'bg-gradient-to-r from-amber-600 to-orange-700 text-white shadow-lg shadow-amber-500/22 shadow-orange-600/20 border-amber-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-amber-500/10 hover:text-amber-400 hover:border-amber-500/20'}"
        onclick={() => onTabChange("flashcards")}
        title={collapsed ? t("nav.flashcards") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {activeTab === 'flashcards' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
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
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold {activeTab === 'flashcards' ? 'text-white' : ''}">{t("nav.flashcards")}</span>
          </div>
        {/if}
      </button>

      <!-- 2. Transcribe -->
      {#if !aiStore.killSwitchActive}
        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
          'transcribe'
            ? 'bg-gradient-to-r from-teal-700 to-emerald-700 text-white shadow-lg shadow-teal-500/20 shadow-emerald-600/20 border-teal-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-teal-500/10 hover:text-teal-400 hover:border-teal-500/20'}"
          onclick={() => onTabChange("transcribe")}
          title={collapsed ? t("nav.transcribe") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {activeTab === 'transcribe' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
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
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold {activeTab === 'transcribe' ? 'text-white' : ''}">{t("nav.transcribe")}</span>
            </div>
          {/if}
        </button>
      {/if}

      <!-- 3. Translate -->
      {#if !aiStore.killSwitchActive}
        <button
          class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
          'translate'
            ? 'bg-gradient-to-r from-fuchsia-700 to-rose-700 text-white shadow-lg shadow-fuchsia-500/20 shadow-rose-600/20 border-fuchsia-500/30 bg-clip-padding'
            : 'text-gray-400 hover:bg-fuchsia-500/10 hover:text-fuchsia-400 hover:border-fuchsia-500/20'}"
          onclick={() => onTabChange("translate")}
          title={collapsed ? t("nav.translate") : undefined}
        >
          <div class="w-9 h-9 rounded-xl {activeTab === 'translate' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
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
            <div class="text-left flex-1 min-w-0 leading-tight">
              <span class="block text-[15px] font-semibold">{t("nav.translate")}</span>
            </div>
          {/if}
        </button>
      {/if}

      <!-- 4. Synchronize -->
      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
        'sync'
          ? 'bg-gradient-to-r from-sky-700 to-cyan-700 text-white shadow-lg shadow-cyan-500/20 shadow-sky-600/20 border-sky-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-sky-500/10 hover:text-sky-400 hover:border-sky-500/20'}"
        onclick={() => onTabChange("sync")}
        title={collapsed ? t("nav.sync") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {activeTab === 'sync' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
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
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold">{t("nav.sync")}</span>
          </div>
        {/if}
      </button>

      <!-- 5. Revise -->
      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
        'align'
          ? 'bg-gradient-to-r from-violet-700 to-indigo-700 text-white shadow-lg shadow-violet-500/20 shadow-indigo-600/20 border-violet-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-violet-500/10 hover:text-violet-400 hover:border-violet-500/20'}"
        onclick={() => onTabChange("align")}
        title={collapsed ? t("nav.revision") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {activeTab === 'align' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16m-7 6h7"/>
          </svg>
        </div>
        {#if !collapsed}
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold">{t("nav.revision")}</span>
          </div>
        {/if}
      </button>

      <!-- 6. Annotate -->
      <button
        class="w-full flex gap-3.5 items-center {collapsed ? 'px-2 justify-center' : 'px-3.5'} h-[60px] rounded-xl transition-all duration-300 border border-transparent cursor-pointer {activeTab ===
        'refine'
          ? 'bg-gradient-to-r from-rose-600 to-pink-700 text-white shadow-lg shadow-rose-500/22 shadow-pink-600/20 border-rose-500/30 bg-clip-padding'
          : 'text-gray-400 hover:bg-rose-500/10 hover:text-rose-400 hover:border-rose-500/20'}"
        onclick={() => onTabChange("refine")}
        title={collapsed ? t("nav.refine") : undefined}
      >
        <div class="w-9 h-9 rounded-xl {activeTab === 'refine' ? 'bg-white/20' : 'bg-white/5'} flex items-center justify-center flex-shrink-0 relative transition-colors border border-white/5">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 8l1.5-1.5M19 4l1.5 1.5M16 5l1.5-1.5" />
          </svg>
        </div>
        {#if !collapsed}
          <div class="text-left flex-1 min-w-0 leading-tight">
            <span class="block text-[15px] font-semibold {activeTab === 'refine' ? 'text-white' : ''}">{t("nav.refine")}</span>
          </div>
        {/if}
      </button>
    {/if}

    <!-- Expert Mode + AI Kill Switch at the bottom of the nav -->
    <div class="mt-auto pt-2 flex flex-col gap-2">
      <button
        type="button"
        onclick={() => uiMode.toggleExpertMode()}
        class="w-full flex h-[60px] items-center {collapsed ? 'justify-center px-2' : 'justify-between px-3.5'} rounded-xl border border-transparent bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-300 transition-all duration-300 cursor-pointer"
        title={collapsed ? (uiMode.expertMode ? t("nav.expertModeOn") : t("nav.expertModeOff")) : undefined}
      >
        <div class="flex items-center gap-3.5">
          <div class="w-9 h-9 rounded-xl bg-white/5 text-gray-400 border border-white/5 flex items-center justify-center flex-shrink-0 relative transition-colors">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.5 6h9.75M10.5 6a1.5 1.5 0 11-3 0m3 0a1.5 1.5 0 10-3 0M3.75 6H7.5m3 12h9.75m-9.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-3.75 0H7.5m9-6h3.75m-3.75 0a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m-9.75 0h9.75" />
            </svg>
          </div>
          {#if !collapsed}
            <span class="text-[15px] font-semibold select-none leading-none text-gray-300">{t("nav.expertMode")}</span>
          {/if}
        </div>
        {#if !collapsed}
          <div
            class="w-10 h-6 rounded-full p-1 transition-colors duration-200 shrink-0 {uiMode.expertMode ? 'bg-indigo-600' : 'bg-white/10'}"
            role="switch"
            aria-checked={uiMode.expertMode}
          >
            <div class="bg-white w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 {uiMode.expertMode ? 'translate-x-4' : 'translate-x-0'}"></div>
          </div>
        {/if}
      </button>

      <button
        type="button"
        onclick={() => {
          if (!aiStore.hasActiveAiProcess) {
            aiStore.toggleKillSwitch();
          }
        }}
        disabled={aiStore.hasActiveAiProcess}
        class="w-full flex h-[60px] items-center {collapsed ? 'justify-center px-2' : 'justify-between px-3.5'} rounded-xl border border-transparent bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-300 transition-all duration-300 {aiStore.hasActiveAiProcess ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}"
        title={collapsed ? (aiStore.killSwitchActive ? t("nav.aiKillSwitchOn") : t("nav.aiKillSwitchOff")) : undefined}
      >
        <div class="flex items-center gap-3.5">
          <div class="w-9 h-9 rounded-xl bg-white/5 text-gray-400 border border-white/5 flex items-center justify-center flex-shrink-0 relative transition-colors">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
            </svg>
            {#if aiStore.hasActiveAiProcess}
              <span class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-blue-500 animate-ping"></span>
            {/if}
          </div>
          {#if !collapsed}
            <span class="text-[15px] font-semibold select-none leading-none text-gray-300">{t("nav.aiKillSwitch") || "AI Kill Switch"}</span>
          {/if}
        </div>
        {#if !collapsed}
          <div 
            class="w-10 h-6 rounded-full p-1 transition-colors duration-200 shrink-0 {aiStore.killSwitchActive ? 'bg-indigo-600' : 'bg-white/10'}"
            role="switch"
            aria-checked={aiStore.killSwitchActive}
          >
            <div class="bg-white w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 {aiStore.killSwitchActive ? 'translate-x-4' : 'translate-x-0'}"></div>
          </div>
        {/if}
      </button>
    </div>
  </nav>

  <div class="h-[92px] px-4 border-t border-white/10 bg-gray-900 flex items-center shrink-0">
    <div class="w-full {collapsed ? 'flex items-center justify-center' : ''}">
      {#if collapsed}
        <a href={repoUrl} target="_blank" class="flex-shrink-0 transition-all hover:scale-110 active:scale-95 duration-150 flex items-center justify-center w-8 h-8 rounded-lg hover:bg-white/5 text-gray-400 hover:text-white border border-transparent hover:border-white/5" title={t("about.repository") || "Repository GitHub"}>
          <svg class="w-6 h-6 shrink-0" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.05A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd" />
          </svg>
        </a>
      {:else}
        <div class="flex items-center justify-between w-full">
          <div class="flex items-center gap-2 min-w-0">
            <a href={authorUrl} target="_blank" class="flex-shrink-0 transition-transform hover:scale-110 active:scale-95 duration-150 inline-block" title="Pierpaolo Spadafora">
              <img src={authorIconUrl} alt="Pierpaolo Spadafora" class="w-8 h-8 rounded-full border border-white/10 shadow-sm" />
            </a>
            <div class="flex flex-col gap-0.5 min-w-0">
              <a href={releasesUrl} target="_blank" class="text-sm font-semibold text-gray-200 hover:text-indigo-400 transition-colors truncate leading-none">
                {appVersionNum || "v0.14.0-dev.1"}
              </a>
              <a href={licenseUrl} target="_blank" class="text-[11px] text-gray-400 hover:text-indigo-400 transition-colors leading-none">
                {appLicense || "GPL-3.0"}
              </a>
            </div>
          </div>
          
          <a href={repoUrl} target="_blank" class="flex items-center gap-1.5 px-2 py-1.5 rounded-lg hover:bg-white/5 text-gray-400 hover:text-white transition-all duration-150 text-right shrink-0 select-none border border-transparent hover:border-white/5">
            <div class="flex flex-col text-[10px] font-bold leading-tight uppercase tracking-wider text-right">
              <span>GitHub</span>
              <span class="opacity-75">Repo</span>
            </div>
            <svg class="w-6 h-6 shrink-0" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path fill-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.05A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clip-rule="evenodd" />
            </svg>
          </a>
        </div>
      {/if}
    </div>
  </div>
</aside>

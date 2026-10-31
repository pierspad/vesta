<script lang="ts">
  import { onMount } from "svelte";
  import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
  import { locale } from "$lib/i18n";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PluginRenderer from "$lib/components/PluginRenderer.svelte";
  import MarkdownContent from "$lib/components/MarkdownContent.svelte";
  import type { InstalledPlugin, PluginInspectResult } from "$lib/services/pluginClient";

  interface Props {
    active?: boolean;
    activePluginId?: string | null;
    onGoToTab?: (tab: string) => void;
  }

  let { active = true, activePluginId = null, onGoToTab }: Props = $props();

  let t = $derived($locale);

  let selectedPluginId = $state<string | null>(null);

  $effect(() => {
    if (activePluginId !== undefined) {
      selectedPluginId = activePluginId;
    }
  });

  let currentPlugin = $derived(
    selectedPluginId ? pluginsStore.plugins.find((p) => p.id === selectedPluginId) ?? null : null
  );

  let urlInput = $state("");
  let refInput = $state("");
  let inspectResult = $state<PluginInspectResult | null>(null);
  let inspecting = $state(false);
  let installing = $state(false);
  let searchQuery = $state("");
  const urlPlaceholder = "https://github.com/owner/repo or /path/to/plugin";

  let uninstallTarget = $state<InstalledPlugin | null>(null);
  let settingsTarget = $state<InstalledPlugin | null>(null);
  let editSettingsValues = $state<Record<string, any>>({});

  let infoTarget = $state<InstalledPlugin | null>(null);
  let showInfoModal = $state(false);

  let contextMenuPlugin = $state<InstalledPlugin | null>(null);
  let contextMenuPos = $state<{ x: number; y: number } | null>(null);

  onMount(() => {
    if (!pluginsStore.initialized) {
      void pluginsStore.init();
    }
  });

  let filteredPlugins = $derived(
    pluginsStore.plugins.filter((p) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        p.name.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q) ||
        (p.description && p.description.toLowerCase().includes(q))
      );
    })
  );

  async function handleBrowseFolder() {
    const selected = await guardedOpen({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === "string") {
      urlInput = selected;
    }
  }

  async function handleInspect() {
    if (!urlInput.trim()) return;
    inspecting = true;
    inspectResult = null;
    try {
      inspectResult = await pluginsStore.inspect(urlInput.trim(), refInput.trim() || undefined);
    } finally {
      inspecting = false;
    }
  }

  async function handleConfirmInstall() {
    if (!inspectResult) return;
    installing = true;
    try {
      await pluginsStore.install(urlInput.trim(), refInput.trim() || undefined);
      inspectResult = null;
      urlInput = "";
      refInput = "";
    } finally {
      installing = false;
    }
  }

  function handleCancelInspect() {
    inspectResult = null;
  }

  function handleOpenPlugin(plugin: InstalledPlugin) {
    if (!plugin.enabled) return;
    selectedPluginId = plugin.id;
    onGoToTab?.("plugin:" + plugin.id);
  }

  function handleBackToPlugins() {
    selectedPluginId = null;
    onGoToTab?.("plugins");
  }

  function openInfoModal(plugin: InstalledPlugin) {
    infoTarget = plugin;
    showInfoModal = true;
    contextMenuPlugin = null;
    contextMenuPos = null;
  }

  function openUninstallConfirm(plugin: InstalledPlugin) {
    uninstallTarget = plugin;
    contextMenuPlugin = null;
    contextMenuPos = null;
  }

  async function confirmUninstall() {
    if (!uninstallTarget) return;
    const id = uninstallTarget.id;
    if (selectedPluginId === id) {
      selectedPluginId = null;
      onGoToTab?.("plugins");
    }
    uninstallTarget = null;
    await pluginsStore.uninstall(id);
  }

  function openSettings(plugin: InstalledPlugin) {
    settingsTarget = plugin;
    editSettingsValues = { ...plugin.settings };
    if (plugin.manifest.settings) {
      for (const field of plugin.manifest.settings) {
        if (editSettingsValues[field.id] === undefined && field.defaultValue !== undefined) {
          editSettingsValues[field.id] = field.defaultValue;
        }
      }
    }
    contextMenuPlugin = null;
    contextMenuPos = null;
  }

  async function savePluginSettings() {
    if (!settingsTarget) return;
    const id = settingsTarget.id;
    const values = { ...editSettingsValues };
    settingsTarget = null;
    await pluginsStore.saveSettings(id, values);
  }

  async function handleUpdate(plugin: InstalledPlugin) {
    contextMenuPlugin = null;
    contextMenuPos = null;
    await pluginsStore.update(plugin.id);
  }

  function formatPermission(perm: string): string {
    if (perm === "network") return t("plugins.permNetwork");
    if (perm === "filesystem:downloads") return t("plugins.permDownloads");
    if (perm.startsWith("filesystem:")) return perm.replace("filesystem:", "Filesystem: ");
    return perm;
  }

  function formatDate(iso?: string | null): string {
    if (!iso) return "—";
    try {
      const d = new Date(iso);
      return isNaN(d.getTime())
        ? iso
        : d.toLocaleDateString(undefined, {
            year: "numeric",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
          });
    } catch {
      return iso;
    }
  }
</script>

<div class="tab-layout flex flex-col h-full bg-gray-900 text-gray-100 select-none">
  {#if currentPlugin}
    <!-- Embedded Plugin View with Top Bar -->
    <div class="px-6 py-3 border-b border-white/10 bg-gray-900 flex items-center justify-between shrink-0 gap-4">
      <div class="flex items-center gap-3 min-w-0">
        <button
          type="button"
          onclick={handleBackToPlugins}
          class="btn-secondary h-9 px-3 text-xs font-semibold inline-flex items-center gap-2 text-gray-300 hover:text-white cursor-pointer shrink-0"
          title={t("plugins.backToPlugins")}
          aria-label={t("plugins.backToPlugins")}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
          <span>{t("plugins.backToPlugins")}</span>
        </button>

        <div class="h-5 w-px bg-white/10 shrink-0"></div>

        <div class="flex items-center gap-2.5 min-w-0">
          <div class="w-7 h-7 rounded-lg bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 flex items-center justify-center shrink-0">
            {#if currentPlugin.manifest.tab?.icon === "film"}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
            {:else if currentPlugin.manifest.tab?.icon === "search"}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
            {/if}
          </div>
          <h2 class="text-sm font-bold text-white truncate">{currentPlugin.name}</h2>
          <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300 shrink-0">v{currentPlugin.version}</span>
        </div>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <button
          type="button"
          class="btn-secondary h-8 w-8 p-0 inline-flex items-center justify-center text-gray-300 hover:text-white rounded-lg transition-colors cursor-pointer"
          onclick={() => openInfoModal(currentPlugin!)}
          title={t("plugins.info")}
          aria-label={t("plugins.info")}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </button>

        {#if currentPlugin.manifest.settings && currentPlugin.manifest.settings.length > 0}
          <button
            type="button"
            class="btn-secondary h-8 w-8 p-0 inline-flex items-center justify-center text-gray-300 hover:text-white rounded-lg transition-colors cursor-pointer"
            onclick={() => openSettings(currentPlugin!)}
            title={t("common.settings")}
            aria-label={t("common.settings")}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        {/if}
      </div>
    </div>

    <!-- Embedded Renderer Body -->
    <div class="flex-1 min-h-0 overflow-hidden relative">
      <PluginRenderer pluginId={currentPlugin.id} active={true} />
    </div>
  {:else}
    <!-- Content Band (Scrollable) -->
    <div class="flex-1 min-h-0 overflow-y-auto p-6 flex flex-col gap-6 scrollbar-thin">
      <!-- Install Card -->
      <div class="rounded-2xl border border-white/10 bg-white/[0.02] p-5 flex flex-col gap-4 shadow-sm">
        <div class="flex items-center justify-between">
          <h2 class="text-sm font-semibold text-white flex items-center gap-2">
            <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
            </svg>
            {t("plugins.installFromRepo")}
          </h2>
          <span class="text-xs text-gray-400">{t("plugins.repoTypes")}</span>
        </div>

        <div class="flex flex-col sm:flex-row gap-3">
          <div class="flex-1 flex gap-2">
            <input
              type="text"
              placeholder={urlPlaceholder}
              bind:value={urlInput}
              class="flex-1 h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
            />
            <button
              type="button"
              onclick={handleBrowseFolder}
              class="btn-secondary h-10 px-3.5 rounded-xl inline-flex items-center gap-2 text-sm text-gray-200 hover:text-white cursor-pointer shrink-0"
              title={t("plugins.browse")}
              aria-label={t("plugins.browse")}
            >
              <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
              </svg>
              <span>{t("plugins.browse")}</span>
            </button>
          </div>

          <input
            type="text"
            placeholder={t("plugins.refPlaceholder")}
            bind:value={refInput}
            class="w-full sm:w-44 h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
          />

          <button
            type="button"
            disabled={!urlInput.trim() || inspecting}
            onclick={handleInspect}
            class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer shrink-0"
          >
            {#if inspecting}
              <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
              {t("plugins.inspecting")}
            {:else}
              {t("plugins.inspect")}
            {/if}
          </button>
        </div>

        <!-- Permission confirmation inspect banner/modal -->
        {#if inspectResult}
          <div class="mt-2 p-5 rounded-xl border border-amber-500/30 bg-amber-500/10 flex flex-col gap-4 animate-in fade-in duration-150">
            <div class="flex items-start justify-between">
              <div class="flex flex-col gap-1">
                <div class="flex items-center gap-2.5">
                  <span class="text-base font-bold text-white">{inspectResult.manifest.name}</span>
                  <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{inspectResult.manifest.version}</span>
                  <span class="text-xs text-gray-400 font-mono">({inspectResult.manifest.id})</span>
                </div>
                {#if inspectResult.manifest.description}
                  <p class="text-xs text-gray-300 mt-1">{inspectResult.manifest.description}</p>
                {/if}
                {#if inspectResult.manifest.author}
                  <span class="text-xs text-gray-400">{t("plugins.author")}: {inspectResult.manifest.author}</span>
                {/if}
              </div>
              <button
                type="button"
                onclick={handleCancelInspect}
                aria-label={t("common.close")}
                class="text-gray-400 hover:text-white transition-colors p-1"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
              </button>
            </div>

            <!-- Permissions list -->
            <div class="flex flex-col gap-1.5">
              <span class="text-xs font-semibold text-amber-300 uppercase tracking-wider">{t("plugins.requestedPermissions")}:</span>
              {#if inspectResult.manifest.permissions.length === 0}
                <span class="text-xs text-gray-400 italic">{t("plugins.noPermissions")}</span>
              {:else}
                <div class="flex flex-wrap gap-2 pt-1">
                  {#each inspectResult.manifest.permissions as perm}
                    <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/20 text-amber-200 border border-amber-400/30">
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
                      {formatPermission(perm)}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>

            <!-- Untrusted code warning -->
            <div class="flex items-start gap-3 p-3 rounded-xl bg-black/30 border border-amber-500/20 text-xs text-amber-200/90 leading-relaxed">
              <svg class="w-5 h-5 text-amber-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
              <div>
                <strong class="font-semibold">{t("plugins.untrustedCodeWarningTitle")}:</strong>
                {t("plugins.untrustedCodeWarning")}
              </div>
            </div>

            <div class="flex items-center justify-end gap-3 pt-1">
              <button
                type="button"
                class="btn-secondary text-xs px-4 py-2 cursor-pointer"
                onclick={handleCancelInspect}
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                disabled={installing}
                onclick={handleConfirmInstall}
                class="btn-primary text-xs px-5 py-2 font-semibold inline-flex items-center gap-2 cursor-pointer"
              >
                {#if installing}
                  <svg class="animate-spin h-3.5 w-3.5 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                  {t("plugins.installing")}
                {:else}
                  {t("plugins.confirmInstall")}
                {/if}
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Installed Plugins Section -->
      <div class="flex flex-col gap-4">
        <!-- Section Header with Integrated Search Filter -->
        <div class="flex items-center justify-between gap-4 flex-wrap">
          <h2 class="text-sm font-semibold text-white">
            {t("plugins.installedList")} ({pluginsStore.plugins.length})
          </h2>

          <div class="relative w-64">
            <input
              type="text"
              placeholder={t("plugins.searchPlaceholder")}
              bind:value={searchQuery}
              class="w-full h-9 pl-9 pr-3 rounded-xl border border-white/10 bg-white/5 text-xs text-gray-200 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
            />
            <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
            </div>
          </div>
        </div>

        {#if pluginsStore.loading && pluginsStore.plugins.length === 0}
          <div class="flex flex-col gap-3">
            <div class="skeleton w-full h-24 rounded-xl"></div>
            <div class="skeleton w-full h-24 rounded-xl"></div>
          </div>
        {:else if filteredPlugins.length === 0}
          <div class="empty-state flex flex-col items-center justify-center p-12 text-center rounded-2xl border border-white/5 bg-white/[0.01]">
            <div class="w-14 h-14 rounded-2xl bg-white/5 flex items-center justify-center text-gray-500 mb-3">
              <svg class="w-7 h-7" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
            </div>
            <h3 class="text-sm font-semibold text-gray-300">
              {searchQuery ? t("plugins.noSearchResults") : t("plugins.noInstalled")}
            </h3>
            <p class="text-xs text-gray-500 max-w-sm mt-1">
              {searchQuery ? t("plugins.clearSearchPrompt") : t("plugins.installPrompt")}
            </p>
          </div>
        {:else}
          <div class="grid grid-cols-1 gap-4">
            {#each filteredPlugins as plugin (plugin.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="p-5 rounded-2xl border border-white/10 bg-white/[0.03] hover:border-white/20 transition-all flex flex-col sm:flex-row sm:items-center justify-between gap-4 cursor-pointer"
                onclick={(e) => {
                  const target = e.target as HTMLElement;
                  if (target.closest("button") || target.closest("a") || target.closest("input")) return;
                  if (plugin.enabled) handleOpenPlugin(plugin);
                }}
                oncontextmenu={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  contextMenuPlugin = plugin;
                  contextMenuPos = { x: e.clientX, y: e.clientY };
                }}
              >
                <div class="flex items-start gap-4 min-w-0 flex-1">
                  <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 border {plugin.enabled ? 'bg-indigo-500/10 border-indigo-500/30 text-indigo-400' : 'bg-white/5 border-white/10 text-gray-500'}">
                    {#if plugin.manifest.tab?.icon === 'film'}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
                    {:else if plugin.manifest.tab?.icon === 'search'}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                    {:else}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
                    {/if}
                  </div>
                  <div class="flex flex-col min-w-0 flex-1">
                    <div class="flex items-center gap-2 flex-wrap">
                      <span class="text-base font-bold text-white truncate">{plugin.name}</span>
                      <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{plugin.version}</span>
                      {#if plugin.enabled}
                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{t("plugins.enabled")}</span>
                      {:else}
                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/10 text-gray-400 border border-white/5">{t("plugins.disabled")}</span>
                      {/if}
                    </div>
                    {#if plugin.description}
                      <p class="text-xs text-gray-400 mt-1 line-clamp-2">{plugin.description}</p>
                    {/if}
                    <div class="flex items-center gap-4 mt-2 text-xs text-gray-500 flex-wrap">
                      <span class="truncate">ID: <code class="text-gray-400 font-mono">{plugin.id}</code></span>
                      <span class="truncate">{t("plugins.installedSource")}: <span class="text-gray-400 font-mono truncate" title={plugin.sourceUrl}>{plugin.sourceUrl}</span></span>
                    </div>
                  </div>
                </div>

                <!-- Action buttons -->
                <div class="flex items-center gap-2 shrink-0 self-end sm:self-center flex-wrap">
                  {#if plugin.enabled}
                    <button
                      type="button"
                      class="btn-primary text-xs px-3.5 py-1.5 font-semibold cursor-pointer"
                      onclick={() => handleOpenPlugin(plugin)}
                    >
                      {t("plugins.open")}
                    </button>
                  {/if}

                  <!-- Info Button -->
                  <button
                    type="button"
                    class="btn-secondary h-8 w-8 p-0 inline-flex items-center justify-center text-gray-300 hover:text-white rounded-lg transition-colors cursor-pointer"
                    onclick={() => openInfoModal(plugin)}
                    title={t("plugins.info")}
                    aria-label={t("plugins.info")}
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                  </button>

                  <!-- Settings Button -->
                  {#if plugin.manifest.settings && plugin.manifest.settings.length > 0}
                    <button
                      type="button"
                      class="btn-secondary h-8 w-8 p-0 inline-flex items-center justify-center text-gray-300 hover:text-white rounded-lg transition-colors cursor-pointer"
                      onclick={() => openSettings(plugin)}
                      title={t("common.settings")}
                      aria-label={t("common.settings")}
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      </svg>
                    </button>
                  {/if}

                  <!-- Update Button -->
                  <button
                    type="button"
                    class="btn-secondary text-xs px-3 py-1.5 cursor-pointer"
                    onclick={() => handleUpdate(plugin)}
                    title={t("plugins.checkUpdates")}
                  >
                    {t("plugins.update")}
                  </button>

                  <!-- Neutral Enable / Disable Button -->
                  {#if plugin.enabled}
                    <button
                      type="button"
                      class="btn-secondary text-xs px-3 py-1.5 cursor-pointer"
                      onclick={() => pluginsStore.disable(plugin.id)}
                    >
                      {t("plugins.disable")}
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="btn-secondary text-xs px-3 py-1.5 cursor-pointer"
                      onclick={() => pluginsStore.enable(plugin.id)}
                    >
                      {t("plugins.enable")}
                    </button>
                  {/if}

                  <!-- Danger Uninstall Button -->
                  <button
                    type="button"
                    class="p-2 rounded-lg text-red-400 hover:text-red-300 hover:bg-red-500/10 transition-colors cursor-pointer"
                    onclick={() => openUninstallConfirm(plugin)}
                    title={t("plugins.uninstall")}
                    aria-label={t("plugins.uninstall")}
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                    </svg>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Fixed Bottom Band (Identical h-[92px] layout across Vesta) -->
  <div class="footer-band h-[92px] px-6 border-t border-white/10 bg-gray-900 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3 text-xs text-gray-400">
      {#if currentPlugin}
        <span class="font-semibold text-gray-200">{currentPlugin.name}</span>
        <span>•</span>
        <span class="font-mono text-gray-400">v{currentPlugin.version}</span>
        <span>•</span>
        <span class="text-emerald-400">{t("plugins.active")}</span>
      {:else}
        <span>{pluginsStore.plugins.length} {t("plugins.totalInstalled")}</span>
        <span>•</span>
        <span class="text-emerald-400">{pluginsStore.enabledPlugins.length} {t("plugins.active")}</span>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      {#if currentPlugin}
        <button
          type="button"
          class="btn-secondary text-xs px-3 py-1.5 font-semibold inline-flex items-center gap-1.5 cursor-pointer"
          onclick={handleBackToPlugins}
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/></svg>
          {t("plugins.backToPlugins")}
        </button>
      {/if}
      <a
        href="https://github.com/pierspad/vesta/blob/main/docs/PLUGINS.md"
        target="_blank"
        rel="noopener noreferrer"
        onclick={(e) => { e.preventDefault(); void openUrl("https://github.com/pierspad/vesta/blob/main/docs/PLUGINS.md"); }}
        class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors inline-flex items-center gap-1 cursor-pointer"
      >
        {t("plugins.developerDocs")} →
      </a>
    </div>
  </div>

  <!-- Right-Click Context Menu -->
  {#if contextMenuPlugin && contextMenuPos}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onclick={() => { contextMenuPlugin = null; contextMenuPos = null; }}
      oncontextmenu={(e) => { e.preventDefault(); contextMenuPlugin = null; contextMenuPos = null; }}
    >
      <div
        class="absolute min-w-[180px] overflow-hidden rounded-xl border border-white/10 bg-gray-900/98 p-1.5 shadow-2xl z-50 text-xs backdrop-blur-md"
        style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
        onclick={(e) => e.stopPropagation()}
      >
        {#if contextMenuPlugin.enabled}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; handleOpenPlugin(p); }}
          >
            <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
            {t("plugins.open")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
          onclick={() => { const p = contextMenuPlugin!; openInfoModal(p); }}
        >
          <svg class="w-4 h-4 text-sky-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          {t("plugins.info")}
        </button>
        {#if contextMenuPlugin.manifest.settings && contextMenuPlugin.manifest.settings.length > 0}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; openSettings(p); }}
          >
            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
            {t("common.settings")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
          onclick={() => { const p = contextMenuPlugin!; handleUpdate(p); }}
        >
          <svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
          {t("plugins.update")}
        </button>
        <div class="my-1 border-t border-white/10"></div>
        {#if contextMenuPlugin.enabled}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; pluginsStore.disable(p.id); }}
          >
            <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
            {t("plugins.disable")}
          </button>
        {:else}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; pluginsStore.enable(p.id); }}
          >
            <svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
            {t("plugins.enable")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-rose-400 hover:bg-rose-500/10 hover:text-rose-300 flex items-center gap-2 cursor-pointer"
          onclick={() => { const p = contextMenuPlugin!; openUninstallConfirm(p); }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
          {t("common.delete")}
        </button>
      </div>
    </div>
  {/if}

  <!-- Extended Plugin Info Modal -->
  {#if showInfoModal && infoTarget}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onclick={() => { showInfoModal = false; infoTarget = null; }}
      onkeydown={(e) => { if (e.key === 'Escape') { showInfoModal = false; infoTarget = null; } }}
    >
      <div
        class="w-full max-w-2xl max-h-[85vh] rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl overflow-hidden"
        onclick={(e) => e.stopPropagation()}
      >
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-white/10 pb-4 shrink-0">
          <div class="flex items-center gap-3 min-w-0">
            <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 flex items-center justify-center shrink-0">
              <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
            </div>
            <div class="min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <h3 class="text-base font-bold text-white truncate">{infoTarget.name} — {t("plugins.info")}</h3>
                <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{infoTarget.version}</span>
                {#if infoTarget.enabled}
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{t("plugins.enabled")}</span>
                {:else}
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/10 text-gray-400 border border-white/5">{t("plugins.disabled")}</span>
                {/if}
              </div>
              <p class="text-xs text-gray-400 font-mono mt-0.5 truncate">ID: {infoTarget.id}</p>
            </div>
          </div>
          <button
            type="button"
            class="text-gray-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/5 cursor-pointer"
            aria-label={t("common.close")}
            onclick={() => { showInfoModal = false; infoTarget = null; }}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <!-- Content Scroll -->
        <div class="flex-1 min-h-0 overflow-y-auto pr-1 flex flex-col gap-5 scrollbar-thin">
          <!-- Metadata Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-4 rounded-xl border border-white/5 bg-white/[0.02] text-xs">
            {#if infoTarget.author}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.author")}</span>
                <span class="text-gray-200">{infoTarget.author}</span>
              </div>
            {/if}
            <div class="flex flex-col gap-0.5">
              <span class="text-gray-500 font-medium">{t("plugins.installedSource")}</span>
              <span class="text-gray-200 truncate font-mono" title={infoTarget.sourceUrl}>{infoTarget.sourceUrl}</span>
            </div>
            <div class="flex flex-col gap-0.5">
              <span class="text-gray-500 font-medium">{t("plugins.installedAt")}</span>
              <span class="text-gray-200">{formatDate(infoTarget.installedAt)}</span>
            </div>
            {#if infoTarget.updatedAt}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.updatedAt")}</span>
                <span class="text-gray-200">{formatDate(infoTarget.updatedAt)}</span>
              </div>
            {/if}
            {#if infoTarget.manifest.homepage}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.homepage")}</span>
                <a
                  href={infoTarget.manifest.homepage}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => { e.preventDefault(); void openUrl(infoTarget!.manifest.homepage!); }}
                  class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
                >
                  {infoTarget.manifest.homepage}
                  <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                </a>
              </div>
            {/if}
            {#if infoTarget.manifest.repository}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.repository")}</span>
                <a
                  href={infoTarget.manifest.repository}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => { e.preventDefault(); void openUrl(infoTarget!.manifest.repository!); }}
                  class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
                >
                  {infoTarget.manifest.repository}
                  <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                </a>
              </div>
            {/if}
          </div>

          <!-- Permissions Section -->
          <div class="flex flex-col gap-2">
            <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.requestedPermissions")}</span>
            {#if !infoTarget.manifest.permissions || infoTarget.manifest.permissions.length === 0}
              <span class="text-xs text-gray-500 italic">{t("plugins.noPermissions")}</span>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#each infoTarget.manifest.permissions as perm}
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/15 text-amber-300 border border-amber-400/25">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
                    {formatPermission(perm)}
                  </span>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Short Description -->
          {#if infoTarget.description}
            <div class="flex flex-col gap-1.5">
              <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.description")}</span>
              <p class="text-sm text-gray-200 leading-relaxed">{infoTarget.description}</p>
            </div>
          {/if}

          <!-- README / Long Description -->
          {#if infoTarget.manifest.longDescription || infoTarget.readme || infoTarget.manifest.readme}
            {@const readmeText = infoTarget.manifest.longDescription || infoTarget.readme || infoTarget.manifest.readme || ""}
            <div class="flex flex-col gap-2 pt-2 border-t border-white/10">
              <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.readme")}</span>
              <div class="p-4 rounded-xl border border-white/5 bg-black/30 overflow-x-auto text-xs">
                <MarkdownContent text={readmeText} />
              </div>
            </div>
          {/if}
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-3 pt-3 border-t border-white/10 shrink-0">
          <button
            type="button"
            class="btn-secondary text-xs px-5 py-2 font-semibold cursor-pointer"
            onclick={() => { showInfoModal = false; infoTarget = null; }}
          >
            {t("common.close")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Uninstall confirmation dialog -->
  {#if uninstallTarget}
    <ConfirmDialog
      show={true}
      title={t("plugins.uninstallConfirmTitle")}
      message={`${t("plugins.uninstallConfirmMessage")} '${uninstallTarget.name}'?`}
      confirmText={t("common.delete")}
      cancelText={t("common.cancel")}
      variant="danger"
      on:confirm={confirmUninstall}
      on:cancel={() => uninstallTarget = null}
    />
  {/if}

  <!-- Plugin Settings Modal -->
  {#if settingsTarget && settingsTarget.manifest.settings}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div class="w-full max-w-lg rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl">
        <div class="flex items-center justify-between border-b border-white/10 pb-3">
          <h3 class="text-base font-bold text-white">{settingsTarget.name} — {t("common.settings")}</h3>
          <button
            type="button"
            class="text-gray-400 hover:text-white cursor-pointer p-1"
            aria-label={t("common.close")}
            onclick={() => settingsTarget = null}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto scrollbar-thin">
          {#each settingsTarget.manifest.settings as field}
            {#if field.type === "checkbox"}
              <button
                type="button"
                role="checkbox"
                aria-checked={Boolean(editSettingsValues[field.id])}
                class="flex items-center gap-3 p-3 rounded-xl border transition-all text-left cursor-pointer {Boolean(editSettingsValues[field.id]) ? 'border-indigo-500/50 bg-indigo-500/10 text-white' : 'border-white/10 bg-white/[0.03] text-gray-300 hover:bg-white/[0.06]'}"
                onclick={() => editSettingsValues[field.id] = !editSettingsValues[field.id]}
              >
                <div class="w-5 h-5 rounded-lg border flex items-center justify-center transition-colors {Boolean(editSettingsValues[field.id]) ? 'border-indigo-400 bg-indigo-600 text-white' : 'border-white/20 bg-black/20 text-transparent'}">
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/></svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-medium">{field.label}</span>
                  {#if field.description}
                    <span class="text-xs text-gray-400">{field.description}</span>
                  {/if}
                </div>
              </button>
            {:else}
              <div class="flex flex-col gap-1.5">
                <label for="modal-setting-{field.id}" class="text-xs font-medium text-gray-400">{field.label}</label>
                <input
                  id="modal-setting-{field.id}"
                  type={field.type === "password" ? "password" : field.type === "number" ? "number" : "text"}
                  bind:value={editSettingsValues[field.id]}
                  placeholder={field.description ?? ""}
                  class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
                />
                {#if field.description}
                  <span class="text-xs text-gray-500">{field.description}</span>
                {/if}
              </div>
            {/if}
          {/each}
        </div>

        <div class="flex justify-end gap-3 pt-3 border-t border-white/10">
          <button
            type="button"
            class="btn-secondary text-xs px-4 py-2 cursor-pointer"
            onclick={() => settingsTarget = null}
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            class="btn-primary text-xs px-5 py-2 font-semibold cursor-pointer"
            onclick={savePluginSettings}
          >
            {t("plugins.save")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

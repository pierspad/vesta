<script lang="ts">
  import { onMount } from "svelte";
  import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
  import { locale } from "$lib/i18n";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import type { InstalledPlugin, PluginInspectResult } from "$lib/services/pluginClient";

  interface Props {
    active?: boolean;
    onGoToTab?: (tab: string) => void;
  }

  let { active = true, onGoToTab }: Props = $props();

  let t = $derived($locale);

  let urlInput = $state("");
  let refInput = $state("");
  let inspectResult = $state<PluginInspectResult | null>(null);
  let inspecting = $state(false);
  let installing = $state(false);
  let searchQuery = $state("");

  let uninstallTarget = $state<InstalledPlugin | null>(null);
  let settingsTarget = $state<InstalledPlugin | null>(null);
  let editSettingsValues = $state<Record<string, any>>({});

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

  function openUninstallConfirm(plugin: InstalledPlugin) {
    uninstallTarget = plugin;
  }

  async function confirmUninstall() {
    if (!uninstallTarget) return;
    const id = uninstallTarget.id;
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
  }

  async function savePluginSettings() {
    if (!settingsTarget) return;
    const id = settingsTarget.id;
    const values = { ...editSettingsValues };
    settingsTarget = null;
    await pluginsStore.saveSettings(id, values);
  }

  function formatPermission(perm: string): string {
    if (perm === "network") return t("plugins.permNetwork");
    if (perm === "filesystem:downloads") return t("plugins.permDownloads");
    if (perm.startsWith("filesystem:")) return perm.replace("filesystem:", "Filesystem: ");
    return perm;
  }
</script>

<div class="tab-layout flex flex-col h-full bg-gray-900 text-gray-100 select-none">
  <!-- Top Command Band -->
  <div class="px-6 py-4 border-b border-white/10 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 flex items-center justify-center">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/>
        </svg>
      </div>
      <div>
        <h1 class="text-lg font-bold text-white leading-tight">{t("plugins.title")}</h1>
        <p class="text-xs text-gray-400 mt-0.5">{t("plugins.subtitle")}</p>
      </div>
    </div>

    <!-- Search filter -->
    <div class="relative w-64">
      <input
        type="text"
        placeholder={t("plugins.searchPlaceholder")}
        bind:value={searchQuery}
        class="w-full h-9 pl-9 pr-3 rounded-xl border border-white/10 bg-white/5 text-xs text-gray-200 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none"
      />
      <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
      </div>
    </div>
  </div>

  <!-- Content Band (Scrollable) -->
  <div class="flex-1 min-h-0 overflow-y-auto p-6 flex flex-col gap-6">
    <!-- Install Card -->
    <div class="rounded-2xl border border-white/10 bg-white/[0.02] p-5 flex flex-col gap-4 shadow-sm">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-white flex items-center gap-2">
          <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
          {t("plugins.installFromRepo")}
        </h2>
        <span class="text-xs text-gray-400">{t("plugins.repoTypes")}</span>
      </div>

      <div class="flex flex-col sm:flex-row gap-3">
        <input
          type="text"
          placeholder="https://github.com/owner/repository"
          bind:value={urlInput}
          class="flex-1 h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
        />
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
          class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer"
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
                <span class="text-xs text-gray-400">({inspectResult.manifest.id})</span>
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
              class="btn-secondary text-xs px-4 py-2"
              onclick={handleCancelInspect}
            >
              {t("common.cancel")}
            </button>
            <button
              type="button"
              disabled={installing}
              onclick={handleConfirmInstall}
              class="btn-primary text-xs px-5 py-2 font-semibold inline-flex items-center gap-2"
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
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-white">
          {t("plugins.installedList")} ({pluginsStore.plugins.length})
        </h2>
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
            <div class="p-5 rounded-2xl border border-white/10 bg-white/[0.03] hover:border-white/20 transition-all flex flex-col sm:flex-row sm:items-center justify-between gap-4">
              <div class="flex items-start gap-4 min-w-0 flex-1">
                <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 border {plugin.enabled ? 'bg-indigo-500/10 border-indigo-500/30 text-indigo-400' : 'bg-white/5 border-white/10 text-gray-500'}">
                  <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
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
                  <div class="flex items-center gap-4 mt-2 text-xs text-gray-500">
                    <span class="truncate">ID: <code class="text-gray-400 font-mono">{plugin.id}</code></span>
                    {#if plugin.manifest.tab}
                      <span class="text-indigo-400 flex items-center gap-1">
                        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"/></svg>
                        {t("plugins.tabPrefix")}: {plugin.manifest.tab.title}
                      </span>
                    {/if}
                  </div>
                </div>
              </div>

              <!-- Action buttons -->
              <div class="flex items-center gap-2 shrink-0 self-end sm:self-center flex-wrap">
                {#if plugin.enabled && plugin.manifest.tab && onGoToTab}
                  <button
                    type="button"
                    class="btn-primary text-xs px-3 py-1.5 font-semibold"
                    onclick={() => onGoToTab?.("plugin:" + plugin.id)}
                  >
                    {t("plugins.openTab")}
                  </button>
                {/if}

                {#if plugin.manifest.settings && plugin.manifest.settings.length > 0}
                  <button
                    type="button"
                    class="btn-secondary text-xs px-3 py-1.5"
                    onclick={() => openSettings(plugin)}
                    title={t("common.settings")}
                  >
                    {t("common.settings")}
                  </button>
                {/if}

                <button
                  type="button"
                  class="btn-secondary text-xs px-3 py-1.5"
                  onclick={() => pluginsStore.update(plugin.id)}
                  title={t("plugins.checkUpdates")}
                >
                  {t("plugins.update")}
                </button>

                {#if plugin.enabled}
                  <button
                    type="button"
                    class="btn-secondary text-xs px-3 py-1.5 text-amber-300 hover:text-amber-200"
                    onclick={() => pluginsStore.disable(plugin.id)}
                  >
                    {t("plugins.disable")}
                  </button>
                {:else}
                  <button
                    type="button"
                    class="btn-secondary text-xs px-3 py-1.5 text-emerald-400 hover:text-emerald-300"
                    onclick={() => pluginsStore.enable(plugin.id)}
                  >
                    {t("plugins.enable")}
                  </button>
                {/if}

                <button
                  type="button"
                  class="p-1.5 rounded-lg text-gray-400 hover:text-rose-400 hover:bg-rose-500/10 transition-colors"
                  onclick={() => openUninstallConfirm(plugin)}
                  title={t("plugins.uninstall")}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Bottom Band (Standard 76px) -->
  <div class="footer-band h-[76px] px-6 border-t border-white/10 bg-gray-950 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3 text-xs text-gray-400">
      <span>{pluginsStore.plugins.length} {t("plugins.totalInstalled")}</span>
      <span>•</span>
      <span class="text-emerald-400">{pluginsStore.enabledPlugins.length} {t("plugins.active")}</span>
    </div>
    <div class="flex items-center gap-3">
      <a
        href="https://github.com/pierspad/vesta/blob/main/docs/PLUGINS.md"
        target="_blank"
        class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors"
      >
        {t("plugins.developerDocs")} →
      </a>
    </div>
  </div>

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
            class="text-gray-400 hover:text-white"
            aria-label={t("common.close")}
            onclick={() => settingsTarget = null}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto">
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
            class="btn-secondary text-xs px-4 py-2"
            onclick={() => settingsTarget = null}
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            class="btn-primary text-xs px-5 py-2 font-semibold"
            onclick={savePluginSettings}
          >
            {t("plugins.save")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

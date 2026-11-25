<script lang="ts">
  import { githubReleaseEndpoint, newerPluginRelease } from "$lib/services/pluginReleases";
  import { updateCheckerStore } from "$lib/stores/updateCheckerStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { onMount, untrack } from "svelte";
  import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
  import { getItem, setItem } from "$lib/config/vestaConfig";
  import { locale } from "$lib/i18n";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import { settledScroll } from "$lib/utils/settledScroll";
  import PluginRenderer from "$lib/components/PluginRenderer.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import SearchInput from "$lib/components/SearchInput.svelte";
  import type { InstalledPlugin, PluginInspectResult } from "$lib/services/pluginClient";
  import PluginIcon from "./plugins/PluginIcon.svelte";
  import PluginInstallCard from "./plugins/PluginInstallCard.svelte";
  import PluginListItem from "./plugins/PluginListItem.svelte";
  import PluginInstallDialog from "./plugins/PluginInstallDialog.svelte";
  import PluginInfoModal from "./plugins/PluginInfoModal.svelte";
  import PluginSettingsModal from "./plugins/PluginSettingsModal.svelte";
  import PluginContextMenu from "./plugins/PluginContextMenu.svelte";
  import PluginSdkModal from "./plugins/PluginSdkModal.svelte";

  interface Props {
    active?: boolean;
    activePluginId?: string | null;
    onGoToTab?: (tab: string) => void;
  }

  let { active = true, activePluginId = null, onGoToTab }: Props = $props();

  let t = $derived($locale);

  let selectedPluginId = $state<string | null>(null);
  $effect(() => {
    if (activePluginId !== undefined) selectedPluginId = activePluginId;
  });

  let currentPlugin = $derived(
    selectedPluginId ? pluginsStore.plugins.find((p) => p.id === selectedPluginId) ?? null : null
  );

  let installMode = $state<"repo" | "folder">("repo");
  let urlInput = $state("");
  let inspectResult = $state<PluginInspectResult | null>(null);
  let inspecting = $state(false);
  let inspectOpener = $state<HTMLElement | null>(null);
  let inspectError = $state(false);
  let installing = $state(false);
  let installError = $state<string | null>(null);
  let searchQuery = $state("");

  let uninstallTarget = $state<InstalledPlugin | null>(null);
  let settingsTarget = $state<InstalledPlugin | null>(null);
  let editSettingsValues = $state<Record<string, any>>({});

  let infoTarget = $state<InstalledPlugin | null>(null);
  let showInfoModal = $state(false);
  let showSdkModal = $state(false);
  let selectedPluginScreen = $state<string>("main");
  let contextMenuPlugin = $state<InstalledPlugin | null>(null);
  let contextMenuPos = $state<{ x: number; y: number } | null>(null);

  onMount(() => {
    if (!pluginsStore.initialized) void pluginsStore.init();
  });

  const pluginOrderKey = "vesta-plugin-order";
  function readPluginOrder(): string[] {
    try {
      const saved: unknown = JSON.parse(getItem(pluginOrderKey) ?? "[]");
      return Array.isArray(saved) ? saved.filter((id): id is string => typeof id === "string") : [];
    } catch {
      return [];
    }
  }
  let pluginOrder = $state(readPluginOrder());
  let draggedPlugin = $state<string | null>(null);
  let dropTarget = $state<string | null>(null);
  let orderedPlugins = $derived.by(() => {
    const positions = new Map(pluginOrder.map((id, index) => [id, index]));
    return [...pluginsStore.plugins].sort((a, b) =>
      (positions.get(a.id) ?? pluginOrder.length) - (positions.get(b.id) ?? pluginOrder.length)
    );
  });

  function movePlugin(id: string, target: string) {
    const order = orderedPlugins.map((plugin) => plugin.id);
    const from = order.indexOf(id);
    const to = order.indexOf(target);
    if (from < 0 || to < 0 || from === to) return;
    order.splice(from, 1);
    order.splice(to, 0, id);
    pluginOrder = order;
    setItem(pluginOrderKey, JSON.stringify(order));
  }

  function finishPluginDrag() {
    draggedPlugin = null;
    dropTarget = null;
  }

  function movePluginWithKey(event: KeyboardEvent, id: string) {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    event.stopPropagation();
    const index = filteredPlugins.findIndex((plugin) => plugin.id === id);
    const target = filteredPlugins[index + (event.key === "ArrowUp" ? -1 : 1)];
    if (target) movePlugin(id, target.id);
  }

  let filteredPlugins = $derived(
    orderedPlugins.filter((p) => {
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
    const selected = await guardedOpen({ directory: true, multiple: false });
    if (selected && typeof selected === "string") urlInput = selected;
  }

  async function handleInspect(event: MouseEvent) {
    if (!urlInput.trim()) return;
    inspectOpener = event.currentTarget as HTMLElement;
    inspecting = true;
    inspectResult = null;
    inspectError = false;
    installError = null;
    try {
      inspectResult = await pluginsStore.inspect(urlInput.trim(), undefined);
    } catch {
      inspectError = true;
    } finally {
      inspecting = false;
    }
  }

  async function handleConfirmInstall() {
    if (!inspectResult || installing) return;
    installing = true;
    installError = null;
    try {
      await pluginsStore.install(inspectResult.source.originalUrl, undefined);
      inspectResult = null;
      urlInput = "";
    } catch {
      installError = pluginsStore.error;
    } finally {
      installing = false;
    }
  }

  function handleCancelInspect() {
    if (!installing) inspectResult = null;
  }

  function handleOpenPlugin(plugin: InstalledPlugin, screen = "main") {
    if (!plugin.enabled) return;
    selectedPluginScreen = screen;
    selectedPluginId = plugin.id;
    onGoToTab?.("plugin:" + plugin.id);
  }

  function handleBackToPlugins() {
    selectedPluginId = null;
    selectedPluginScreen = "main";
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
    contextMenuPlugin = null;
    contextMenuPos = null;
    if (plugin.manifest.tab?.settingsScreen) {
      handleOpenPlugin(plugin, plugin.manifest.tab.settingsScreen);
    } else if (plugin.manifest.settings && plugin.manifest.settings.length > 0) {
      settingsTarget = plugin;
      editSettingsValues = { ...plugin.settings };
      for (const field of plugin.manifest.settings) {
        if (editSettingsValues[field.id] === undefined && field.defaultValue !== undefined) {
          editSettingsValues[field.id] = field.defaultValue;
        }
      }
    } else if (plugin.enabled) {
      handleOpenPlugin(plugin, "settings");
    }
  }

  async function savePluginSettings() {
    if (!settingsTarget) return;
    const id = settingsTarget.id;
    const values = { ...editSettingsValues };
    settingsTarget = null;
    await pluginsStore.saveSettings(id, values);
  }

  const onlinePlugins = $derived(pluginsStore.plugins.filter((plugin) => plugin.enabled && githubReleaseEndpoint(plugin)));
  let checkedOnline = $state(false);
  let checkingOnline = $state(false);
  let availableReleases = $state<Record<string, string>>({});

  $effect(() => {
    if (active && !currentPlugin && updateCheckerStore.automaticUpdateChecks && onlinePlugins.length && !checkedOnline) {
      untrack(() => { void checkPluginReleases(false); });
    }
  });

  async function checkPluginReleases(manual: boolean) {
    if (checkingOnline) return;
    checkedOnline = true;
    checkingOnline = true;
    try {
      const releases: Record<string, string> = {};
      let failures = false;
      for (const plugin of onlinePlugins) {
        try {
          const tag = await newerPluginRelease(plugin);
          if (tag) releases[plugin.id] = tag;
        } catch {
          failures = true;
        }
      }
      availableReleases = releases;
      if (manual && failures) snackbar.show(t("settings.updatesCheckFailed"), "warning");
      else if (manual && !Object.keys(releases).length) snackbar.show(t("settings.updatesUpToDate"), "success");
    } finally {
      checkingOnline = false;
    }
  }

  async function handleUpdate(plugin: InstalledPlugin) {
    contextMenuPlugin = null;
    contextMenuPos = null;
    const tag = availableReleases[plugin.id];
    if (!tag && githubReleaseEndpoint(plugin)) {
      await checkPluginReleases(true);
      return;
    }
    await pluginsStore.update(plugin.id, tag);
    if (tag && pluginsStore.plugins.some((item) => item.id === plugin.id && `v${item.version.replace(/^v/, "")}` === tag)) {
      delete availableReleases[plugin.id];
    }
  }
</script>

<div class="tab-layout flex flex-col h-full bg-gray-900 text-gray-100 select-none">
  {#if currentPlugin}
    {#key `${currentPlugin.id}:${selectedPluginScreen}`}
      <PluginRenderer pluginId={currentPlugin.id} screen={selectedPluginScreen} onExit={handleBackToPlugins}>
        {#snippet header(actions, back, busy)}
          <div class="h-[89px] px-6 border-b border-white/10 bg-gray-900 flex items-center justify-between shrink-0 gap-4">
            <div class="flex items-center gap-3 min-w-0">
              <button
                type="button"
                onclick={back}
                disabled={busy}
                class="btn-icon h-9 w-9 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                title={t("setup.back")}
                aria-label={t("setup.back")}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
                </svg>
              </button>
              <div class="h-5 w-px bg-white/10 shrink-0"></div>
              <div class="flex items-center gap-2.5 min-w-0">
                <div class="w-7 h-7 rounded-lg bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 flex items-center justify-center shrink-0 overflow-hidden">
                  <PluginIcon manifest={currentPlugin.manifest} iconUrl={currentPlugin.iconUrl} sizeClass="w-4 h-4" />
                </div>
                <h2 class="text-sm font-bold text-white truncate">{currentPlugin.name}</h2>
                <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300 shrink-0">v{currentPlugin.version}</span>
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0">{@render actions()}</div>
          </div>
        {/snippet}
        {#snippet fallbackFooter()}
          <div class="flex items-center gap-3 text-xs text-gray-400">
            <span class="font-semibold text-gray-200">{currentPlugin.name}</span>
            <span>•</span>
            <span class="font-mono text-gray-400">v{currentPlugin.version}</span>
            <span>•</span>
            <span class="text-emerald-400">{t("plugins.active")}</span>
          </div>
          <div class="flex items-center gap-3">
            <button
              type="button"
              onclick={() => (showSdkModal = true)}
              title={t("plugins.developerDocs")}
              class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors inline-flex items-center gap-1 cursor-pointer"
            >
              {t("plugins.developerDocs")} →
            </button>
          </div>
        {/snippet}
      </PluginRenderer>
    {/key}
  {:else}
    <div use:settledScroll class="flex-1 min-h-0 overflow-y-auto overscroll-contain p-6 flex flex-col gap-6 scrollbar-thin [scrollbar-gutter:stable]">
      <PluginInstallCard
        bind:installMode
        bind:urlInput
        {inspecting}
        {inspectError}
        onBrowseFolder={handleBrowseFolder}
        onInspect={handleInspect}
      />

      <div class="flex flex-col gap-4">
        <div class="flex items-center justify-between gap-4 flex-wrap">
          <h2 class="text-sm font-semibold text-white">
            {t("plugins.installedList")} ({pluginsStore.plugins.length})
          </h2>

          {#if onlinePlugins.length}
            <button type="button" class="btn-secondary btn-compact inline-flex items-center gap-1.5" disabled={checkingOnline}
              onclick={() => checkPluginReleases(true)}>
              <svg aria-hidden="true" class="h-3.5 w-3.5 shrink-0" class:animate-spin={checkingOnline} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
              </svg>
              {t(checkingOnline ? "settings.updatesChecking" : "plugins.checkUpdates")}
            </button>
          {/if}
          <SearchInput bind:value={searchQuery} placeholder={t("plugins.searchPlaceholder")} class="w-64" />
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
              <PluginListItem
                {plugin}
                canReorder={filteredPlugins.length > 1}
                isDragged={draggedPlugin === plugin.id}
                isDropTarget={dropTarget === plugin.id}
                availableRelease={availableReleases[plugin.id]}
                onOpen={() => handleOpenPlugin(plugin)}
                onEnable={() => pluginsStore.enable(plugin.id)}
                onDisable={() => pluginsStore.disable(plugin.id)}
                onUpdate={() => void handleUpdate(plugin)}
                onInfo={() => openInfoModal(plugin)}
                onSettings={() => openSettings(plugin)}
                onUninstall={() => openUninstallConfirm(plugin)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  contextMenuPlugin = plugin;
                  contextMenuPos = { x: e.clientX, y: e.clientY };
                }}
                onDragStart={() => { draggedPlugin = plugin.id; }}
                onDragOver={(event) => {
                  if (!draggedPlugin || draggedPlugin === plugin.id) return;
                  event.preventDefault();
                  event.stopPropagation();
                  if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
                  dropTarget = plugin.id;
                }}
                onDragLeave={(event) => {
                  if (dropTarget === plugin.id && !(event.relatedTarget instanceof Node && (event.currentTarget as HTMLElement).contains(event.relatedTarget))) {
                    dropTarget = null;
                  }
                }}
                onDrop={(event) => {
                  if (!draggedPlugin) return;
                  event.preventDefault();
                  event.stopPropagation();
                  movePlugin(draggedPlugin, plugin.id);
                  finishPluginDrag();
                }}
                onDragEnd={finishPluginDrag}
                onMoveKey={(event) => movePluginWithKey(event, plugin.id)}
              />
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <FooterActions>
      {#snippet left()}
        <div class="flex items-center gap-3 text-xs text-gray-400">
          <span>{pluginsStore.plugins.length} {t("plugins.totalInstalled")}</span>
          <span>•</span>
          <span class="text-emerald-400">{pluginsStore.enabledPlugins.length} {t("plugins.active")}</span>
        </div>
      {/snippet}
      {#snippet right()}
        <div class="flex items-center gap-3">
          <button
            type="button"
            onclick={() => (showSdkModal = true)}
            title={t("plugins.developerDocs")}
            class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors inline-flex items-center gap-1 cursor-pointer"
          >
            {t("plugins.developerDocs")} →
          </button>
        </div>
      {/snippet}
    </FooterActions>
  {/if}

  {#if active && !selectedPluginId && inspectResult}
    <PluginInstallDialog
      {inspectResult}
      {installing}
      {installError}
      {inspectOpener}
      onCancel={handleCancelInspect}
      onConfirm={handleConfirmInstall}
    />
  {/if}

  {#if showSdkModal}
    <PluginSdkModal onClose={() => (showSdkModal = false)} />
  {/if}

  {#if contextMenuPlugin && contextMenuPos}
    <PluginContextMenu
      plugin={contextMenuPlugin}
      pos={contextMenuPos}
      onClose={() => { contextMenuPlugin = null; contextMenuPos = null; }}
      onOpen={(p) => handleOpenPlugin(p)}
      onInfo={openInfoModal}
      onSettings={openSettings}
      onUpdate={(p) => void handleUpdate(p)}
      onEnable={(p) => pluginsStore.enable(p.id)}
      onDisable={(p) => pluginsStore.disable(p.id)}
      onUninstall={openUninstallConfirm}
    />
  {/if}

  {#if showInfoModal && infoTarget}
    <PluginInfoModal plugin={infoTarget} onClose={() => { showInfoModal = false; infoTarget = null; }} />
  {/if}

  {#if uninstallTarget}
    <ConfirmDialog
      show={true}
      title={t("plugins.uninstallConfirmTitle")}
      message={`${t("plugins.uninstallConfirmMessage")} '${uninstallTarget.name}'?`}
      confirmText={t("common.delete")}
      cancelText={t("common.cancel")}
      variant="danger"
      on:confirm={confirmUninstall}
      on:cancel={() => (uninstallTarget = null)}
    />
  {/if}

  {#if settingsTarget && settingsTarget.manifest.settings}
    <PluginSettingsModal
      plugin={settingsTarget}
      bind:values={editSettingsValues}
      onClose={() => (settingsTarget = null)}
      onSave={savePluginSettings}
    />
  {/if}
</div>

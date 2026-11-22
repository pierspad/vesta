<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { InstalledPlugin } from "$lib/services/pluginClient";
  import PluginIcon from "./PluginIcon.svelte";

  interface Props {
    plugin: InstalledPlugin;
    canReorder: boolean;
    isDragged: boolean;
    isDropTarget: boolean;
    availableRelease?: string;
    onOpen: () => void;
    onEnable: () => void;
    onDisable: () => void;
    onUpdate: () => void;
    onInfo: () => void;
    onSettings: () => void;
    onUninstall: () => void;
    onContextMenu: (event: MouseEvent) => void;
    onDragStart: () => void;
    onDragOver: (event: DragEvent) => void;
    onDragLeave: (event: DragEvent) => void;
    onDrop: (event: DragEvent) => void;
    onDragEnd: () => void;
    onMoveKey: (event: KeyboardEvent) => void;
  }

  let {
    plugin,
    canReorder,
    isDragged,
    isDropTarget,
    availableRelease,
    onOpen,
    onEnable,
    onDisable,
    onUpdate,
    onInfo,
    onSettings,
    onUninstall,
    onContextMenu,
    onDragStart,
    onDragOver,
    onDragLeave,
    onDrop,
    onDragEnd,
    onMoveKey,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div
  role="group"
  aria-label={plugin.name}
  class:ring-2={isDropTarget}
  class:ring-indigo-400={isDropTarget}
  class:opacity-50={isDragged}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  class="p-5 rounded-2xl border border-white/10 bg-white/[0.03] transition-colors flex flex-col sm:flex-row sm:items-center justify-between gap-4 {plugin.enabled ? 'hover:border-white/20' : ''}"
  oncontextmenu={onContextMenu}
>
  <div class="relative flex items-start gap-4 min-w-0 flex-1 {plugin.enabled ? '' : 'grayscale opacity-65'}">
    {#if plugin.enabled}
      <button
        type="button"
        class="absolute inset-0 rounded-xl cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-4"
        aria-label={`${t("plugins.open")} · ${plugin.name}`}
        onclick={onOpen}
      ></button>
    {/if}
    {#if canReorder}
      <button
        type="button"
        draggable="true"
        aria-label={`${t("plugins.reorder")} · ${plugin.name}`}
        aria-keyshortcuts="ArrowUp ArrowDown"
        title={t("plugins.reorder")}
        class="relative z-10 inline-flex h-8 w-8 shrink-0 self-center cursor-grab items-center justify-center rounded-lg text-gray-400 hover:bg-white/10 hover:text-white active:cursor-grabbing"
        ondragstart={(event) => {
          if (!event.dataTransfer) return;
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("text/plain", plugin.id);
          onDragStart();
        }}
        ondragend={onDragEnd}
        onkeydown={onMoveKey}
      >
        <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 16 16" fill="currentColor">
          <circle cx="5.5" cy="3.5" r="1.25"/><circle cx="10.5" cy="3.5" r="1.25"/>
          <circle cx="5.5" cy="8" r="1.25"/><circle cx="10.5" cy="8" r="1.25"/>
          <circle cx="5.5" cy="12.5" r="1.25"/><circle cx="10.5" cy="12.5" r="1.25"/>
        </svg>
      </button>
    {/if}
    <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 border overflow-hidden {plugin.enabled ? 'bg-indigo-500/10 border-indigo-500/30 text-indigo-400' : 'bg-white/5 border-white/10 text-gray-500'}">
      <PluginIcon manifest={plugin.manifest} iconUrl={plugin.iconUrl} />
    </div>
    <div class="flex flex-col min-w-0 flex-1">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-base font-bold text-white truncate">{plugin.name}</span>
        <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{plugin.version}</span>
        {#if availableRelease}
          <span class="rounded bg-cyan-500/15 px-2 py-0.5 text-xs text-cyan-300">
            {t("settings.updatesNewVersionAvailable", { version: availableRelease })}
          </span>
        {/if}
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

  <div class="flex items-center gap-2.5 shrink-0 self-end sm:self-center">
    {#if plugin.enabled}
      <button
        type="button"
        class="btn-primary h-8 px-3 text-xs rounded-lg font-semibold inline-flex items-center gap-1.5 cursor-pointer"
        onclick={onOpen}
        title={t("plugins.open")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
        </svg>
        <span>{t("plugins.open")}</span>
      </button>
      <button
        type="button"
        class="text-amber-400 border border-amber-500/25 bg-amber-500/10 hover:bg-amber-500/20 inline-flex items-center gap-1.5 h-8 px-3 text-xs rounded-lg font-medium transition-colors cursor-pointer"
        onclick={onDisable}
        title={t("plugins.disable")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
        <span>{t("plugins.disable")}</span>
      </button>
    {:else}
      <button
        type="button"
        class="text-gray-300 border border-white/15 bg-white/5 hover:bg-white/10 inline-flex items-center gap-1.5 h-8 px-3 text-xs rounded-lg font-medium transition-colors cursor-pointer"
        onclick={onEnable}
        title={t("plugins.enable")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
        <span>{t("plugins.enable")}</span>
      </button>
    {/if}

    <div class="grid grid-cols-2 gap-1.5 pl-1 border-l border-white/10">
      <button
        type="button"
        class="w-7 h-7 text-emerald-400 border border-emerald-500/25 bg-emerald-500/10 hover:bg-emerald-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!plugin.enabled}
        onclick={onUpdate}
        title={t("plugins.update")}
        aria-label={t("plugins.update")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
      </button>
      <button
        type="button"
        class="w-7 h-7 text-sky-400 border border-sky-500/25 bg-sky-500/10 hover:bg-sky-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!plugin.enabled}
        onclick={onInfo}
        title={t("plugins.info")}
        aria-label={t("plugins.info")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      </button>
      <button
        type="button"
        class="w-7 h-7 text-gray-300 border border-white/15 bg-white/5 hover:bg-white/10 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!plugin.enabled}
        onclick={onSettings}
        title={t("common.settings")}
        aria-label={t("common.settings")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </button>
      <button
        type="button"
        class="w-7 h-7 text-rose-400 border border-rose-500/25 bg-rose-500/10 hover:bg-rose-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer"
        onclick={onUninstall}
        title={t("plugins.uninstall")}
        aria-label={t("plugins.uninstall")}
      >
        <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
        </svg>
      </button>
    </div>
  </div>
</div>

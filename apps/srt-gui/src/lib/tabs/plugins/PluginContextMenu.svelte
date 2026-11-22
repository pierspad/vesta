<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { InstalledPlugin } from "$lib/services/pluginClient";

  interface Props {
    plugin: InstalledPlugin;
    pos: { x: number; y: number };
    onClose: () => void;
    onOpen: (plugin: InstalledPlugin) => void;
    onInfo: (plugin: InstalledPlugin) => void;
    onSettings: (plugin: InstalledPlugin) => void;
    onUpdate: (plugin: InstalledPlugin) => void;
    onEnable: (plugin: InstalledPlugin) => void;
    onDisable: (plugin: InstalledPlugin) => void;
    onUninstall: (plugin: InstalledPlugin) => void;
  }

  let {
    plugin,
    pos,
    onClose,
    onOpen,
    onInfo,
    onSettings,
    onUpdate,
    onEnable,
    onDisable,
    onUninstall,
  }: Props = $props();

  let t = $derived($locale);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50"
  onclick={onClose}
  oncontextmenu={(e) => { e.preventDefault(); onClose(); }}
>
  <div
    class="absolute min-w-[180px] overflow-hidden rounded-xl border border-white/10 bg-gray-900/98 p-1.5 shadow-2xl z-50 text-xs backdrop-blur-md"
    style="left: {pos.x}px; top: {pos.y}px;"
    onclick={(e) => e.stopPropagation()}
  >
    {#if plugin.enabled}
      <button
        type="button"
        class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
        onclick={() => { onClose(); onOpen(plugin); }}
      >
        <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
        {t("plugins.open")}
      </button>
    {/if}
    <button
      type="button"
      class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={!plugin.enabled}
      onclick={() => onInfo(plugin)}
    >
      <svg class="w-4 h-4 text-sky-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
      {t("plugins.info")}
    </button>
    {#if plugin.manifest.tab?.settingsScreen || plugin.manifest.settings?.length}
      <button
        type="button"
        class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
        disabled={!plugin.enabled}
        onclick={() => onSettings(plugin)}
      >
        <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
        {t("common.settings")}
      </button>
    {/if}
    <button
      type="button"
      class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
      disabled={!plugin.enabled}
      onclick={() => onUpdate(plugin)}
    >
      <svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
      {t("plugins.update")}
    </button>
    <div class="my-1 border-t border-white/10"></div>
    {#if plugin.enabled}
      <button
        type="button"
        class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
        onclick={() => { onClose(); onDisable(plugin); }}
      >
        <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
        {t("plugins.disable")}
      </button>
    {:else}
      <button
        type="button"
        class="w-full text-start px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
        onclick={() => { onClose(); onEnable(plugin); }}
      >
        <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
        {t("plugins.enable")}
      </button>
    {/if}
    <button
      type="button"
      class="w-full text-start px-3 py-2 rounded-lg text-rose-400 hover:bg-rose-500/10 hover:text-rose-300 flex items-center gap-2 cursor-pointer"
      onclick={() => onUninstall(plugin)}
    >
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
      {t("common.delete")}
    </button>
  </div>
</div>

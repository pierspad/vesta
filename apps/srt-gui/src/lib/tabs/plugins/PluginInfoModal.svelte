<script lang="ts">
  import { locale } from "$lib/i18n";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MarkdownContent from "$lib/components/MarkdownContent.svelte";
  import type { InstalledPlugin } from "$lib/services/pluginClient";
  import PluginIcon from "./PluginIcon.svelte";
  import { formatPermission, formatPluginDate, getPermissionScopes } from "./pluginHelpers";

  interface Props {
    plugin: InstalledPlugin;
    onClose: () => void;
  }

  let { plugin, onClose }: Props = $props();
  let t = $derived($locale);
  let scopes = $derived(getPermissionScopes(plugin.manifest.permissions));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
>
  <div
    class="w-full max-w-2xl max-h-[85vh] rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl overflow-hidden"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="flex items-center justify-between border-b border-white/10 pb-4 shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 flex items-center justify-center shrink-0 overflow-hidden">
          <PluginIcon manifest={plugin.manifest} iconUrl={plugin.iconUrl} />
        </div>
        <div class="min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h3 class="text-base font-bold text-white truncate">{plugin.name} — {t("plugins.info")}</h3>
            <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{plugin.version}</span>
            {#if plugin.enabled}
              <span class="px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{t("plugins.enabled")}</span>
            {:else}
              <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/10 text-gray-400 border border-white/5">{t("plugins.disabled")}</span>
            {/if}
          </div>
          <p class="text-xs text-gray-400 font-mono mt-0.5 truncate">ID: {plugin.id}</p>
        </div>
      </div>
      <button
        type="button"
        class="text-gray-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/5 cursor-pointer"
        aria-label={t("common.close")}
        onclick={onClose}
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1 flex flex-col gap-5 scrollbar-thin">
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-4 rounded-xl border border-white/5 bg-white/[0.02] text-xs">
        {#if plugin.author}
          <div class="flex flex-col gap-0.5">
            <span class="text-gray-500 font-medium">{t("plugins.author")}</span>
            <span class="text-gray-200">{plugin.author}</span>
          </div>
        {/if}
        <div class="flex flex-col gap-0.5">
          <span class="text-gray-500 font-medium">{t("plugins.installedSource")}</span>
          <span class="text-gray-200 truncate font-mono" title={plugin.sourceUrl}>{plugin.sourceUrl}</span>
        </div>
        <div class="flex flex-col gap-0.5">
          <span class="text-gray-500 font-medium">{t("plugins.installedAt")}</span>
          <span class="text-gray-200">{formatPluginDate(plugin.installedAt)}</span>
        </div>
        {#if plugin.updatedAt}
          <div class="flex flex-col gap-0.5">
            <span class="text-gray-500 font-medium">{t("plugins.updatedAt")}</span>
            <span class="text-gray-200">{formatPluginDate(plugin.updatedAt)}</span>
          </div>
        {/if}
        {#if plugin.manifest.homepage}
          <div class="flex flex-col gap-0.5">
            <span class="text-gray-500 font-medium">{t("plugins.homepage")}</span>
            <a
              href={plugin.manifest.homepage}
              target="_blank"
              rel="noopener noreferrer"
              onclick={(e) => { e.preventDefault(); void openUrl(plugin.manifest.homepage!); }}
              class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
            >
              {plugin.manifest.homepage}
              <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
            </a>
          </div>
        {/if}
        {#if plugin.manifest.repository}
          <div class="flex flex-col gap-0.5">
            <span class="text-gray-500 font-medium">{t("plugins.repository")}</span>
            <a
              href={plugin.manifest.repository}
              target="_blank"
              rel="noopener noreferrer"
              onclick={(e) => { e.preventDefault(); void openUrl(plugin.manifest.repository!); }}
              class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
            >
              {plugin.manifest.repository}
              <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
            </a>
          </div>
        {/if}
      </div>

      <div class="flex flex-col gap-2">
        <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.requestedPermissions")}</span>
        {#if scopes.length === 0}
          <span class="text-xs text-gray-500 italic">{t("plugins.noPermissions")}</span>
        {:else}
          <div class="flex flex-wrap gap-2">
            {#each scopes as perm}
              <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/15 text-amber-300 border border-amber-400/25">
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
                {formatPermission(perm, t)}
              </span>
            {/each}
          </div>
        {/if}
      </div>

      {#if plugin.description}
        <div class="flex flex-col gap-1.5">
          <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.description")}</span>
          <p class="text-sm text-gray-200 leading-relaxed">{plugin.description}</p>
        </div>
      {/if}

      {#if plugin.manifest.longDescription || plugin.readme || plugin.manifest.readme}
        {@const readmeText = plugin.manifest.longDescription || plugin.readme || plugin.manifest.readme || ""}
        <div class="flex flex-col gap-2 pt-2 border-t border-white/10">
          <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.readme")}</span>
          <div class="p-4 rounded-xl border border-white/5 bg-black/30 overflow-x-auto text-xs">
            <MarkdownContent text={readmeText} />
          </div>
        </div>
      {/if}
    </div>

    <div class="flex justify-end gap-3 pt-3 border-t border-white/10 shrink-0">
      <button
        type="button"
        class="btn-secondary text-xs px-5 py-2 font-semibold cursor-pointer"
        onclick={onClose}
      >
        {t("common.close")}
      </button>
    </div>
  </div>
</div>

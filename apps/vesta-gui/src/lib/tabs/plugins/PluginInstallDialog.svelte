<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { PluginInspectResult } from "$lib/services/pluginClient";
  import PluginIcon from "./PluginIcon.svelte";
  import { formatPermission, getPermissionScopes } from "./pluginHelpers";

  interface Props {
    inspectResult: PluginInspectResult;
    installing: boolean;
    installError: string | null;
    inspectOpener: HTMLElement | null;
    onCancel: () => void;
    onConfirm: () => void;
  }

  let {
    inspectResult,
    installing,
    installError,
    inspectOpener,
    onCancel,
    onConfirm,
  }: Props = $props();

  let t = $derived($locale);
  let scopes = $derived(getPermissionScopes(inspectResult.manifest.permissions));

  function openInstallDialog(node: HTMLDialogElement) {
    const opener = inspectOpener;
    node.showModal();
    return {
      destroy() {
        node.close();
        if (opener?.isConnected) opener.focus({ preventScroll: true });
      },
    };
  }
</script>

<dialog
  use:openInstallDialog
  aria-labelledby="plugin-install-dialog-title"
  aria-busy={installing}
  oncancel={(event) => { event.preventDefault(); onCancel(); }}
  onclick={(event) => { if (event.target === event.currentTarget) onCancel(); }}
  onkeydown={(event) => event.stopPropagation()}
  class="m-auto max-h-[calc(100dvh_-_2rem)] w-[calc(100%_-_2rem)] max-w-xl overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-0 text-gray-100 shadow-2xl backdrop:bg-black/65"
>
  <div class="flex flex-col gap-4 p-5 sm:p-6">
    <div class="flex items-start justify-between">
      <div class="flex items-center gap-3.5 min-w-0">
        <div class="w-12 h-12 rounded-xl bg-amber-500/20 border border-amber-500/30 text-amber-300 flex items-center justify-center shrink-0 overflow-hidden">
          <PluginIcon manifest={inspectResult.manifest} iconUrl={inspectResult.iconUrl} />
        </div>
        <div class="flex flex-col gap-1 min-w-0">
          <div class="flex items-center gap-2.5 flex-wrap">
            <h2 id="plugin-install-dialog-title" class="text-base font-bold text-white break-words">{inspectResult.manifest.name}</h2>
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
      </div>
      <button
        type="button"
        onclick={onCancel}
        disabled={installing}
        aria-label={t("common.close")}
        class="text-gray-400 hover:text-white transition-colors p-1"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
      </button>
    </div>

    <div class="flex flex-col gap-1.5">
      <span class="text-xs font-semibold text-amber-300 uppercase tracking-wider">{t("plugins.requestedPermissions")}:</span>
      {#if scopes.length === 0}
        <span class="text-xs text-gray-400 italic">{t("plugins.noPermissions")}</span>
      {:else}
        <div class="flex flex-wrap gap-2 pt-1">
          {#each scopes as perm}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/20 text-amber-200 border border-amber-400/30">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
              {formatPermission(perm, t)}
            </span>
          {/each}
        </div>
      {/if}
    </div>

    <div class="flex items-start gap-3 p-3 rounded-xl bg-black/30 border border-amber-500/20 text-xs text-amber-200/90 leading-relaxed">
      <svg class="w-5 h-5 text-amber-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
      <div>
        <strong class="font-semibold">{t("plugins.untrustedCodeWarningTitle")}:</strong>
        {t("plugins.untrustedCodeWarning")}
      </div>
    </div>

    {#if installError}
      <p role="alert" class="break-words text-sm text-red-300">{installError}</p>
    {/if}

    <div class="flex items-center justify-end gap-3 pt-1">
      <button
        type="button"
        class="btn-secondary text-xs px-4 py-2 cursor-pointer"
        onclick={onCancel}
        disabled={installing}
      >
        {t("common.cancel")}
      </button>
      <button
        type="button"
        disabled={installing}
        onclick={onConfirm}
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
</dialog>

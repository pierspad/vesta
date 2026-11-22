<script lang="ts">
  import { locale } from "$lib/i18n";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { autoSyncStore } from "$lib/stores/autoSyncStore.svelte";
  import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
  import type { TranscribeTierEntry } from "$lib/config/transcribeTiers";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import AutoSyncControls from "$lib/components/AutoSyncControls.svelte";
  import SpeechEngineSelector from "$lib/components/SpeechEngineSelector.svelte";

  interface Props {
    active: boolean;
    canReset: boolean;
    canSave: boolean;
    canAutoSync: boolean;
    selectedEngine: TranscribeTierEntry | null | undefined;
    onLoadSession: () => void;
    onStartAutoSync: () => void;
    onRequestReset: () => void;
    onSaveFile: () => void;
  }

  let {
    active,
    canReset,
    canSave,
    canAutoSync,
    selectedEngine = $bindable(),
    onLoadSession,
    onStartAutoSync,
    onRequestReset,
    onSaveFile,
  }: Props = $props();

  let t = $derived($locale);
</script>

<FooterActions>
  {#snippet left()}
    <div class="relative group">
      <button
        onclick={onLoadSession}
        class="px-5 py-2.5 bg-cyan-600 hover:bg-cyan-500 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-cyan-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer"
      >
        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
        </svg>
        {t("sync.loadSession")}
      </button>
      <div class="pointer-events-none absolute bottom-full left-0 z-50 mb-3 rounded-xl border border-cyan-500/30 bg-gray-950/95 p-3 text-center text-xs text-cyan-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
        {t("sync.tooltipLoadSession")}
      </div>
    </div>

    {#if !aiStore.killSwitchActive}
      <div class="w-40 min-[961px]:w-56 max-w-full min-w-0">
        <SpeechEngineSelector
          purpose="sync"
          compact
          disabled={!active || autoSyncStore.isAutoSyncing || localWhisperOperation.busy}
          onresolve={(entry) => { selectedEngine = entry; }}
        />
      </div>
    {/if}
  {/snippet}
  {#snippet center()}
    <AutoSyncControls {canAutoSync} onStart={onStartAutoSync} />
  {/snippet}
  {#snippet right()}
    <div class="relative group">
      <button
        onclick={onRequestReset}
        disabled={!canReset}
        class="px-5 py-2.5 bg-amber-500 hover:bg-amber-400 disabled:bg-amber-500/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-amber-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 disabled:cursor-not-allowed cursor-pointer"
      >
        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        {t("sync.newSync")}
      </button>
      <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
        {t("sync.newSyncDesc")}
      </div>
    </div>

    <div class="relative group">
      <button
        onclick={onSaveFile}
        disabled={!canSave}
        class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 disabled:cursor-not-allowed cursor-pointer"
      >
        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        {t("sync.saveFile")}
      </button>
      <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-green-500/30 bg-gray-950/95 p-3 text-center text-xs text-green-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
        {t("sync.tooltipSaveFile")}
      </div>
    </div>
  {/snippet}
</FooterActions>

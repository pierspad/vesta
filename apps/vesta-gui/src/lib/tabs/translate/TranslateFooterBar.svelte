<script lang="ts">
  import { locale } from "$lib/i18n";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import CopyPathChip from "$lib/components/CopyPathChip.svelte";
  import WorkflowErrorStatus from "$lib/components/WorkflowErrorStatus.svelte";
  import type { TranslateResult } from "$lib/services/translate";

  interface ProgressState {
    current_batch: number;
    total_batches: number;
    percentage: number;
    eta_seconds: number | null;
  }

  interface Props {
    isTranslating: boolean;
    cancelling: boolean;
    progress: ProgressState | null;
    result: TranslateResult | null;
    error: string | null;
    canStart: boolean;
    useTiers: boolean;
    blockedReason: string;
    formatEta: (seconds: number | null) => string;
    onCancel: () => void;
    onStart: () => void;
    onBlockedStart?: () => void;
    onReset: () => void;
  }

  let {
    isTranslating,
    cancelling,
    progress,
    result,
    error,
    canStart,
    useTiers,
    blockedReason,
    formatEta,
    onCancel,
    onStart,
    onBlockedStart,
    onReset,
  }: Props = $props();

  let t = $derived($locale);
</script>

<FooterActions>
  {#snippet background()}
    {#if isTranslating}
      <div
        class="absolute inset-y-0 left-0 bg-gradient-to-r from-purple-500/15 to-pink-500/20 transition-all duration-300 ease-out z-0 pointer-events-none"
        style="width: {progress?.percentage || 0}%"
      ></div>
      <div class="absolute inset-0 bg-shimmer-stripes opacity-15 z-0 pointer-events-none"></div>
    {/if}
  {/snippet}
  {#snippet left()}
    <div class="flex items-center gap-4 select-none z-10 min-w-0 flex-1">
      {#if isTranslating}
        <div class="flex items-center gap-3">
          <div class="w-5 h-5 border-2 border-purple-400 border-t-transparent rounded-full animate-spin"></div>
          <div class="flex flex-col">
            <span class="text-[10px] text-purple-400 font-bold uppercase tracking-wider">{t("translate.translating")}</span>
            <span class="text-xs text-white font-medium truncate max-w-lg">
              {t("translate.batch")} {progress?.current_batch || 0}/{progress?.total_batches || 0} ({Math.round(progress?.percentage || 0)}%)
              {#if progress?.eta_seconds}
                <span class="text-gray-400 ml-2">{t("translate.eta", { time: formatEta(progress.eta_seconds) })}</span>
              {/if}
            </span>
          </div>
        </div>
      {:else if result}
        <div class="flex items-center gap-3">
          {#if result.success}
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-emerald-400 font-bold uppercase tracking-wider">{t("translate.finished")}</span>
              {#if result.output_path}
                <CopyPathChip
                  path={result.output_path}
                  copiedMessage={t("common.pathCopied")}
                  tooltip={t("common.clickToCopyPath")}
                />
              {:else}
                <span class="text-xs text-white font-medium">{result.message}</span>
              {/if}
            </div>
          {:else}
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("translate.error")}</span>
              <span class="text-xs text-red-300 font-medium truncate max-w-lg">{result.message}</span>
            </div>
          {/if}
        </div>
      {:else if error}
        <WorkflowErrorStatus title={t("translate.error")} message={error} warning />
      {:else if blockedReason && useTiers}
        <span class="text-xs text-gray-400" role="status">{t("translate.blocked.selectFiles")}</span>
      {/if}
    </div>
  {/snippet}
  {#snippet right()}
    <div class="flex items-center gap-4 z-10 select-none shrink-0">
      {#if isTranslating}
        <button
          disabled={cancelling}
          onclick={onCancel}
          class="px-5 py-2.5 bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-red-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer"
        >
          <svg class="w-4 h-4 text-red-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
          {t("translate.cancel")}
        </button>
      {:else if result || error}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="relative group {!canStart ? 'cursor-help' : ''}"
          onclick={() => {
            if (!canStart) onBlockedStart?.();
          }}
        >
          <button
            onclick={onStart}
            disabled={!canStart}
            class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 border border-emerald-500/30 disabled:bg-emerald-600/40 disabled:border-white/10 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-900/30 disabled:shadow-none flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-50 {!canStart ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
          >
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {t("translate.retry")}
          </button>
          <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
            {t("translate.retryDesc")}
          </div>
        </div>
        <div class="relative group">
          <button
            onclick={onReset}
            class="px-5 py-2.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 rounded-xl font-bold text-sm transition-all border border-amber-500/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
          >
            <svg class="w-4 h-4 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {t("translate.newTranslation")}
          </button>
          <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
            {t("translate.newTranslationDesc")}
          </div>
        </div>
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="relative group {!canStart ? 'cursor-help' : ''}"
          onclick={() => {
            if (!canStart) onBlockedStart?.();
          }}
        >
          <button
            onclick={onStart}
            disabled={!canStart}
            class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 border border-emerald-500/30 disabled:bg-emerald-600/40 disabled:border-white/10 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-900/30 disabled:shadow-none flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50 {!canStart ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
          >
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            {t("translate.start")}
          </button>
          <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
            {blockedReason || t("translate.start")}
          </div>
        </div>
      {/if}
    </div>
  {/snippet}
</FooterActions>

<style>
  @keyframes progress-stripes {
    0% { background-position: 0 0; }
    100% { background-position: 40px 0; }
  }
  .bg-shimmer-stripes {
    background-image: linear-gradient(
      45deg,
      rgba(139, 92, 246, 0.15) 25%,
      transparent 25%,
      transparent 50%,
      rgba(139, 92, 246, 0.15) 50%,
      rgba(139, 92, 246, 0.15) 75%,
      transparent 75%,
      transparent
    );
    background-size: 40px 40px;
    animation: progress-stripes 1.2s linear infinite;
  }
</style>

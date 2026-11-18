<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { RefineRunSummary } from "$lib/workflows/refineAutoRun";

  interface Props {
    useTiers: boolean;
    tierCounts: { tiers: number; endpoints: number };
    isValidatingLlm: boolean;
    llmErrorText: string | null;
    llmHighlightPulse: boolean;
    operationBusy: boolean;
    onlyUnannotated: boolean;
    useBatchMode: boolean;
    autoRefining: boolean;
    isStopping: boolean;
    hasCards: boolean;
    progressCurrent: number;
    progressFailed: number;
    progressTotal: number;
    runSummary: RefineRunSummary | null;
    logs: string[];
    onEditLlmSettings: () => void;
    onStartAuto: () => void;
    onStopAuto: () => void;
  }

  let {
    useTiers,
    tierCounts,
    isValidatingLlm,
    llmErrorText,
    llmHighlightPulse,
    operationBusy,
    onlyUnannotated = $bindable(),
    useBatchMode = $bindable(),
    autoRefining,
    isStopping,
    hasCards,
    progressCurrent,
    progressFailed,
    progressTotal,
    runSummary,
    logs,
    onEditLlmSettings,
    onStartAuto,
    onStopAuto,
  }: Props = $props();

  let t = $derived($locale);
  let pct = $derived(progressTotal > 0 ? ((progressCurrent + progressFailed) / progressTotal) * 100 : 0);
</script>

<div class="flex flex-col gap-4 flex-1 min-h-0 overflow-y-auto scrollbar-thin pt-3">
  <button
    type="button"
    onclick={onEditLlmSettings}
    class="flex flex-col gap-2 bg-white/5 border border-white/10 hover:bg-white/10 hover:border-white/20 rounded-xl p-3.5 transition-all duration-200 cursor-pointer text-left w-full shrink-0"
    class:llm-requirement-pulse={llmHighlightPulse}
  >
    <div class="flex items-center gap-2 text-xs text-gray-400">
      <svg class="w-4 h-4 text-indigo-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
      </svg>
      <span class="font-semibold text-gray-300">{t("refine.llmTiersLabel")}</span>
      {#if useTiers}
        <span class="text-[10px] bg-indigo-500/10 border border-indigo-500/20 text-indigo-300 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
          {t("refine.llmTiersSummary", { tiers: tierCounts.tiers, endpoints: tierCounts.endpoints })}
        </span>
      {:else}
        <span class="text-[10px] bg-white/5 border border-white/10 text-gray-500 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">—</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 text-xs text-gray-400 flex-wrap">
      <svg class="w-4 h-4 {llmErrorText ? 'text-amber-400' : 'text-emerald-400'} shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
      </svg>
      <span class="font-semibold text-gray-300">{t("refine.llmEngine")}</span>
      {#if isValidatingLlm}
        <span class="text-[10px] bg-white/5 border border-white/10 text-gray-400 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">…</span>
      {:else if !llmErrorText}
        <span class="text-[10px] bg-emerald-500/10 border border-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ml-1">
          {t("refine.llmTiersReady")}
        </span>
      {:else}
        <span class="font-semibold text-amber-400 italic ml-1">{llmErrorText}</span>
      {/if}
      <span class="text-[10px] text-gray-500 ml-auto">{t("refine.llmTiersEdit")}</span>
    </div>
  </button>

  <div class="grid grid-cols-2 gap-3 shrink-0">
    <button
      type="button"
      disabled={operationBusy}
      aria-pressed={onlyUnannotated}
      onclick={() => (onlyUnannotated = !onlyUnannotated)}
      class="flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
        {onlyUnannotated ? 'bg-indigo-500/10 border-indigo-500/50 text-white' : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
    >
      <span class="text-xs font-bold uppercase tracking-wider">{t("refine.options.onlyUnannotated")}</span>
      <div class="w-3.5 h-3.5 rounded-full border border-indigo-400/50 flex items-center justify-center {onlyUnannotated ? 'bg-indigo-500' : ''}">
        {#if onlyUnannotated}
          <svg class="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
        {/if}
      </div>
    </button>

    <button
      type="button"
      disabled={operationBusy}
      aria-pressed={useBatchMode}
      onclick={() => (useBatchMode = !useBatchMode)}
      class="flex items-center justify-between p-3.5 rounded-xl border text-left transition-all duration-200 cursor-pointer select-none
        {useBatchMode ? 'bg-indigo-500/10 border-indigo-500/50 text-white' : 'bg-white/5 border-white/10 text-gray-400 hover:bg-white/10 hover:text-gray-200'}"
    >
      <span class="text-xs font-bold uppercase tracking-wider">{t("refine.options.batchMode")}</span>
      <div class="w-3.5 h-3.5 rounded-full border border-indigo-400/50 flex items-center justify-center {useBatchMode ? 'bg-indigo-500' : ''}">
        {#if useBatchMode}
          <svg class="w-2.5 h-2.5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
        {/if}
      </div>
    </button>
  </div>

  <div class="flex items-center gap-3 mt-2 shrink-0">
    <button
      onclick={onEditLlmSettings}
      class="flex-1 rounded-xl bg-white/5 hover:bg-white/10 border border-white/10 text-xs font-bold text-gray-300 px-4 py-2.5 transition-all cursor-pointer flex items-center justify-center gap-2"
    >
      <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
      </svg>
      {t("refine.btn.editPrompt")}
    </button>

    {#if autoRefining}
      <button
        onclick={onStopAuto}
        disabled={isStopping}
        class="flex-1 rounded-xl bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-xs font-bold text-red-100 px-4 py-2.5 transition-all cursor-pointer flex items-center justify-center gap-2"
      >
        {isStopping ? t("refine.btn.stopping") : t("refine.btn.stop")}
      </button>
    {:else}
      <button
        onclick={onStartAuto}
        disabled={operationBusy || isValidatingLlm || !!llmErrorText || !hasCards}
        class="flex-1 rounded-xl bg-amber-600 hover:bg-amber-500 border border-amber-500/30 disabled:bg-amber-600/40 text-xs font-bold text-white px-4 py-2.5 shadow-lg shadow-amber-950/30 transition-all cursor-pointer flex items-center justify-center gap-2 {(llmErrorText || !hasCards) ? 'opacity-50 cursor-not-allowed' : ''}"
      >
        <svg class="w-4 h-4 text-amber-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
        {t("refine.btn.startAI")}
      </button>
    {/if}
  </div>

  <div class="bg-white/5 border border-white/10 rounded-xl p-3 shrink-0">
    <div class="flex justify-between text-xs text-gray-300 mb-1.5 font-semibold">
      <span>{t("refine.progress.title")}</span>
      <span>{progressCurrent + progressFailed} / {progressTotal} ({Math.round(pct)}%)</span>
    </div>
    <div class="w-full bg-white/10 h-2 rounded-full overflow-hidden">
      <div class="bg-gradient-to-r from-rose-500 to-pink-500 h-full rounded-full transition-all duration-300" style="width: {pct}%"></div>
    </div>
    {#if runSummary}
      <p class="mt-2 text-xs text-gray-300" role="status">
        {t("refine.progress.summary", { success: runSummary.done, failed: runSummary.failed, remaining: runSummary.remaining })}
      </p>
    {/if}
  </div>

  <details class="rounded-xl border border-white/10 bg-white/5 p-3 text-xs text-gray-400">
    <summary class="cursor-pointer">{t("refine.progress.activity")} ({logs.length})</summary>
    <div class="mt-2 h-24 overflow-y-auto [scrollbar-gutter:stable] break-words space-y-1">
      {#each logs as log}<p>{log}</p>{/each}
    </div>
  </details>
</div>

<style>
  :global(.llm-requirement-pulse) {
    animation: llm-requirement-pulse 0.9s ease-in-out 2;
    border-color: rgba(251, 191, 36, 0.75) !important;
    box-shadow:
      0 0 0 1px rgba(251, 191, 36, 0.3),
      0 0 24px rgba(251, 191, 36, 0.24) !important;
  }

  @keyframes llm-requirement-pulse {
    0%, 100% {
      border-color: rgba(251, 191, 36, 0.35);
      box-shadow: 0 0 0 0 rgba(251, 191, 36, 0);
    }
    45% {
      border-color: rgba(251, 191, 36, 0.9);
      box-shadow:
        0 0 0 1px rgba(251, 191, 36, 0.45),
        0 0 28px rgba(251, 191, 36, 0.36);
    }
  }
</style>

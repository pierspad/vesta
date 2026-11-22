<script lang="ts">
  import { locale } from "$lib/i18n";
  import LogPanel, { type LogEntry } from "$lib/panels/LogPanel.svelte";
  import type { TranslateResult } from "$lib/services/translate";

  interface ProgressState {
    current_batch: number;
    total_batches: number;
    percentage: number;
    eta_seconds: number | null;
  }

  interface Props {
    mode?: "progress" | "logs";
    isTranslating?: boolean;
    progress?: ProgressState | null;
    result?: TranslateResult | null;
    error?: string | null;
    logs?: LogEntry[];
    formatEta: (seconds: number | null) => string;
    resultMessage: (r: TranslateResult) => string;
    onClearError?: () => void;
    onClearLogs?: () => void;
  }

  let {
    mode = "progress",
    isTranslating = false,
    progress = null,
    result = null,
    error = null,
    logs = [],
    formatEta,
    resultMessage,
    onClearError,
    onClearLogs,
  }: Props = $props();

  let t = $derived($locale);
</script>

{#if mode === "logs"}
  <LogPanel
    title={t("translate.logs")}
    clearLogText={t("translate.clearLog")}
    noLogText={t("translate.noLog")}
    {logs}
    onclear={() => onClearLogs?.()}
    minHeight="190px"
    maxHeightContent="16rem"
  />
{:else if isTranslating || progress || result || error}
  <div class="space-y-3 shrink-0">
    {#if isTranslating || progress}
      <div class="glass-card p-4 animate-fade-in shrink-0 {isTranslating ? 'animate-pulse-glow' : ''}">
        <div class="flex items-center gap-6">
          <div class="flex-1">
            <div class="progress-modern h-2">
              <div class="progress-modern-bar" style="width: {progress?.percentage || 0}%"></div>
            </div>
          </div>
          <span class="text-gray-400 text-sm whitespace-nowrap">
            {t("translate.batch")} {progress?.current_batch || 0}/{progress?.total_batches || 0}
          </span>
          <span class="text-xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
            {Math.round(progress?.percentage || 0)}%
          </span>
          {#if progress?.eta_seconds}
            <span class="text-gray-500 text-sm flex items-center gap-1">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              {formatEta(progress.eta_seconds)}
            </span>
          {/if}
        </div>
      </div>
    {/if}
    {#if result}
      <div class="glass-card p-4 border-l-4 animate-fade-in shrink-0 {result.success ? 'border-green-500 bg-green-500/5' : 'border-red-500 bg-red-500/5'}">
        <div class="flex items-center gap-3">
          {#if result.success}
            <svg class="w-5 h-5 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
          {:else}
            <svg class="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          {/if}
          <div class="flex-1">
            <p class="{result.success ? 'text-green-400' : 'text-red-400'} font-medium">{resultMessage(result)}</p>
            {#if result.output_path}
              <p class="text-xs text-gray-500 mt-1 font-mono truncate">📁 {result.output_path}</p>
            {/if}
          </div>
        </div>
      </div>
    {/if}
    {#if error}
      <div class="glass-card p-4 border border-red-500/30 bg-red-500/10 animate-fade-in shrink-0">
        <div class="flex items-center gap-3">
          <svg class="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p class="text-red-300 flex-1 text-sm">{error}</p>
          <button onclick={() => onClearError?.()} class="text-red-400 hover:text-red-300">✕</button>
        </div>
      </div>
    {/if}
  </div>
{/if}

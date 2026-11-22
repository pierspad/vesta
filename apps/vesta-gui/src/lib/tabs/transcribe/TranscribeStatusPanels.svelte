<script lang="ts">
  import { locale } from "$lib/i18n";
  import LogPanel, { type LogEntry } from "$lib/panels/LogPanel.svelte";
  import MediaIcon from "$lib/components/MediaIcon.svelte";

  interface Props {
    mode: "progress" | "logs";
    isTranscribing?: boolean;
    progress?: number;
    progressMessage?: string;
    result?: { success: boolean; message: string; output_path?: string } | null;
    error?: string | null;
    logs?: LogEntry[];
    onClearError?: () => void;
    onClearLogs?: () => void;
  }

  let {
    mode,
    isTranscribing = false,
    progress = 0,
    progressMessage = "",
    result = null,
    error = null,
    logs = [],
    onClearError,
    onClearLogs,
  }: Props = $props();

  let t = $derived($locale);
</script>

{#if mode === "progress"}
  <div class="space-y-3">
    {#if isTranscribing || progress > 0}
      <div class="glass-card p-4 shrink-0 {isTranscribing ? 'animate-pulse-glow' : ''}">
        <div class="flex items-center gap-4">
          <div class="flex-1">
            <div class="progress-modern h-2">
              <div class="progress-modern-bar bg-gradient-to-r from-cyan-500 to-blue-500" style="width: {progress}%"></div>
            </div>
          </div>
          <span class="text-lg font-bold text-cyan-400">{progress}%</span>
        </div>
        {#if progressMessage}
          <p class="text-gray-400 text-xs mt-2">{progressMessage}</p>
        {/if}
      </div>
    {/if}
    {#if result}
      <div class="glass-card p-4 shrink-0 border-l-4 {result.success ? 'border-green-500 bg-green-500/5' : 'border-red-500 bg-red-500/5'}">
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
          <p class="{result.success ? 'text-green-400' : 'text-red-400'} font-medium">{result.message}</p>
          {#if result.output_path}
            <p class="text-xs text-gray-500 mt-1 font-mono truncate">
              <MediaIcon kind="folder" /> {result.output_path}
            </p>
          {/if}
        </div>
      </div>
    {/if}
    {#if error}
      <div class="glass-card p-4 shrink-0 border border-red-500/30 bg-red-500/10">
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
{:else}
  <LogPanel
    title={t("transcribe.log")}
    clearLogText={t("transcribe.clearLog")}
    noLogText={t("translate.noLog")}
    {logs}
    onclear={() => onClearLogs?.()}
    minHeight="170px"
    maxHeightContent="100%"
  />
{/if}

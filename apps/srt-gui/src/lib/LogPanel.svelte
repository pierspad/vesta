<script module lang="ts">
  export interface LogEntry {
    id: number;
    timestamp: string;
    message: string;
    type: "info" | "success" | "warning" | "error" | "progress" | "terminal" | string;
    details?: string;
    progressKey?: string;
  }
</script>

<script lang="ts">
  interface Props {
    title: string;
    clearLogText: string;
    noLogText: string;
    logs: LogEntry[];
    onclear: () => void;
    minHeight?: string;
    maxHeightContent?: string;
  }

  let { 
    title, 
    clearLogText, 
    noLogText, 
    logs, 
    onclear,
    minHeight = "180px",
    maxHeightContent = "16rem" // 64 in tailwind is 16rem
  }: Props = $props();

  function logStyle(type: LogEntry["type"]) {
    switch (type) {
      case "success":
        return {
          bg: "bg-green-500/10",
          border: "border-green-500/20",
          text: "text-green-400",
          icon: "✅",
        };
      case "error":
        return {
          bg: "bg-red-500/10",
          border: "border-red-500/20",
          text: "text-red-400",
          icon: "❌",
        };
      case "warning":
        return {
          bg: "bg-yellow-500/10",
          border: "border-yellow-500/20",
          text: "text-yellow-400",
          icon: "⚠️",
        };
      case "progress":
        return {
          bg: "bg-blue-500/10",
          border: "border-blue-500/20",
          text: "text-blue-400",
          icon: "⏳",
        };
      case "terminal":
        return {
          bg: "bg-gray-800/50",
          border: "border-gray-700/50",
          text: "text-gray-300 font-mono",
          icon: "▶",
        };
      default:
        return {
          bg: "bg-gray-500/10",
          border: "border-gray-500/20",
          text: "text-gray-300",
          icon: "ℹ️",
        };
    }
  }
</script>

<div class="glass-card p-4 flex flex-col" style="min-height: {minHeight};">
  <div class="flex items-center justify-between mb-3 shrink-0">
    <h4 class="text-sm font-medium text-gray-400 flex items-center gap-2">
      <svg
        class="w-4 h-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M4 6h16M4 12h16m-7 6h7"
        />
      </svg>
      {title}
    </h4>
    {#if logs.length > 0}
      <button
        onclick={onclear}
        class="text-xs text-gray-500 hover:text-gray-400 transition-colors"
      >
        {clearLogText}
      </button>
    {/if}
  </div>
  <div class="flex-1 min-h-0 overflow-y-auto bg-black/20 rounded-lg p-3" style="max-height: {maxHeightContent};">
    {#if logs.length > 0}
      <div class="space-y-1.5">
        {#each logs as log (log.id)}
          {@const style = logStyle(log.type)}
          <div
            class="p-2 rounded-lg border {style.bg} {style.border} flex items-start gap-2 animate-fade-in"
          >
            <span class="text-xs flex-shrink-0">{style.icon}</span>
            <div class="flex-1 min-w-0">
              <p class="text-xs {style.text} leading-tight break-words whitespace-pre-wrap">
                {log.message}
              </p>
              {#if log.details}
                <p
                  class="text-[10px] text-gray-500 break-words whitespace-pre-wrap mt-0.5"
                  title={log.details}
                >
                  {log.details}
                </p>
              {/if}
            </div>
            <span class="text-[10px] text-gray-600 flex-shrink-0">
              {log.timestamp}
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-gray-600 text-xs">{noLogText}</p>
    {/if}
  </div>
</div>

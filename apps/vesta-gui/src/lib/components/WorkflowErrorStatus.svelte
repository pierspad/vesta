<script lang="ts">
  let { title, message, warning = false }: { title: string; message: string; warning?: boolean } = $props();
</script>

<div class="flex items-center gap-3">
  <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5"
        d={warning ? "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" : "M6 18L18 6M6 6l12 12"} />
    </svg>
  </div>
  <div class="flex flex-col">
    <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{title}</span>
    <span class="text-xs text-red-300 font-medium truncate max-w-lg">{message}</span>
  </div>
</div>

<script lang="ts">
  import type { Snippet } from "svelte";
  let { files, children, fillViewport = false }: { files: Snippet; children: Snippet; fillViewport?: boolean } = $props();
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col gap-4" data-testid="workflow-workspace">
  <div class="min-h-0 max-h-[45%] shrink-0 overflow-y-auto overscroll-contain" data-testid="pinned-files">{@render files()}</div>
  <div class="min-h-0 flex-1 {fillViewport ? 'flex flex-col' : ''} overflow-y-auto overscroll-contain scrollbar-thin pr-2 pb-2" data-testid="settings-scroll" data-workflow-scroll>
    {@render children()}
  </div>
</div>

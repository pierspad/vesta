<script lang="ts">
  import type { Snippet } from "svelte";
  let { files, children }: { files: Snippet; children: Snippet } = $props();
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4" data-testid="workflow-workspace">
  <div class="shrink-0" data-testid="pinned-files">{@render files()}</div>
  <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain scrollbar-thin pr-2 pb-2" data-testid="settings-scroll" data-workflow-scroll>
    {@render children()}
  </div>
</div>

<script lang="ts">
  import type { Snippet } from "svelte";
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import type { PanelId } from "$lib/utils/flashcardLayout";
  let { columns, easyMode, files, panel }: {
    columns: 1 | 2 | 3; easyMode: boolean; files: Snippet; panel: Snippet<[PanelId]>;
  } = $props();
</script>

<WorkflowWorkspace {files}>
  <div class="panels-grid" data-columns={columns} data-easy={easyMode} role="list">
    <div class="relative min-w-0 self-start z-40" role="listitem">{@render panel("audioClips")}</div>
    <div class="secondary">
      <div class="visuals">
        <div class="relative min-w-0 z-30" role="listitem">{@render panel("snapshots")}</div>
        <div class="relative min-w-0 z-20" role="listitem">{@render panel("videoClips")}</div>
      </div>
      {#if !easyMode}<div class="relative min-w-0 z-10" role="listitem">{@render panel("cardFilters")}</div>{/if}
    </div>
  </div>
</WorkflowWorkspace>

<style>
  .panels-grid { display: grid; gap: 1rem; align-items: start; grid-template-columns: minmax(0, 1fr); }
  .secondary, .visuals { display: contents; min-width: 0; }
  .panels-grid[data-columns="2"] { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .panels-grid[data-columns="2"] .secondary,
  .panels-grid[data-columns="2"] .visuals,
  .panels-grid[data-columns="3"] .visuals { display: flex; flex-direction: column; gap: 1rem; }
  .panels-grid[data-columns="3"] { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .panels-grid[data-columns="3"][data-easy="true"] .visuals { display: contents; }
</style>

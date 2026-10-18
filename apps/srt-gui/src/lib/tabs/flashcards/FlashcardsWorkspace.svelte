<script lang="ts">
  import type { Snippet } from "svelte";
  import { SETTINGS_PANEL_IDS, type PanelId } from "$lib/utils/flashcardLayout";
  let { columns, easyMode, files, panel }: {
    columns: 1 | 2 | 3;
    easyMode: boolean;
    files: Snippet;
    panel: Snippet<[PanelId]>;
  } = $props();
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4" data-testid="flashcards-workspace">
  <div class="shrink-0" data-testid="pinned-files">{@render files()}</div>
  <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain scrollbar-thin pr-2 pb-2" data-testid="settings-scroll">
    <div class="panels-grid" data-columns={columns} data-easy={easyMode} role="list">
      {#each SETTINGS_PANEL_IDS as id, index (id)}
        {#if id !== "cardFilters" || !easyMode}
          <div class="relative min-w-0 self-start" style:grid-area={id} style:z-index={40 - index * 5} role="listitem">
            {@render panel(id)}
          </div>
        {/if}
      {/each}
    </div>
  </div>
</div>

<style>
  .panels-grid {
    display: grid;
    gap: 1rem;
    align-items: start;
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas: "audioClips" "snapshots" "videoClips" "cardFilters";
  }
  .panels-grid[data-columns="2"] {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-areas: "audioClips snapshots" "audioClips videoClips" "audioClips cardFilters";
    grid-template-rows: min-content min-content min-content;
  }
  .panels-grid[data-columns="3"] {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-template-areas: "audioClips snapshots cardFilters" "audioClips videoClips cardFilters";
    grid-template-rows: min-content min-content;
  }
  .panels-grid[data-columns="2"][data-easy="true"] {
    grid-template-areas: "audioClips snapshots" "audioClips videoClips";
    grid-template-rows: min-content min-content;
  }
  .panels-grid[data-columns="3"][data-easy="true"] {
    grid-template-areas: "audioClips snapshots videoClips";
    grid-template-rows: min-content;
  }
</style>

<script lang="ts">
  import { uiMode } from "./uiModeStore.svelte.ts";
  import AlignTabNew from "./AlignTabNew.svelte";
  import AlignTabOld from "./AlignTabOld.svelte";

  let { active = false } = $props<{ active?: boolean }>();
</script>

{#if uiMode.useOldReviseUi}
  <AlignTabOld {active} />
{:else}
  <AlignTabNew {active} />
{/if}

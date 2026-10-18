<script lang="ts">
  import FlashcardsWorkspace from "$lib/tabs/flashcards/FlashcardsWorkspace.svelte";
  let columns = $state<1 | 2 | 3>(2);
</script>
<button onclick={() => columns = columns === 2 ? 1 : 3}>Resize</button>
<div class="h-[700px] flex flex-col">
  <FlashcardsWorkspace {columns} easyMode={false}>
    {#snippet files()}<p>Files &amp; Output</p>{/snippet}
    {#snippet panel(id)}
      {#if id === "audioClips"}<input aria-label="Preview position" value="original" />{:else}<p>{id}</p>{/if}
    {/snippet}
  </FlashcardsWorkspace>
</div>

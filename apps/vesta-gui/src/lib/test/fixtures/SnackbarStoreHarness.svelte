<script lang="ts">
  import Snackbar from "$lib/components/Snackbar.svelte";
  import type { SnackbarStore } from "$lib/stores/snackbarStore.svelte";

  let { store }: { store: SnackbarStore } = $props();
</script>

{#if store.message}
  {@const key = store.key}
  <Snackbar
    message={store.message}
    variant={store.variant}
    duration={store.duration}
    animationKey={store.key}
    paused={store.paused}
    onpause={(reason) => store.pause(reason)}
    onresume={(reason) => store.resume(reason)}
    onclose={() => store.close(key)}
  />
{/if}

<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  let { active = false }: { active?: boolean } = $props();
  let operations = $state(0);
  const lifecycle = (window as any).__appTabLifecycle ??= { mounted: 0, destroyed: 0 };
  onMount(() => { lifecycle.mounted += 1; });
  onDestroy(() => { lifecycle.destroyed += 1; });
</script>

<section data-testid="tab-probe" data-active={active}>
  <output data-testid="operation-count">{operations}</output>
  <button data-testid="run-operation" onclick={() => operations += 1}>Run operation</button>
</section>

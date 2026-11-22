<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { recordSupportEvent } from "$lib/utils/supportLog";
  let {
    active = false,
    requestedSection = $bindable(),
    highlightItemId = $bindable(),
  }: { active?: boolean; requestedSection?: string; highlightItemId?: string | null } = $props();
  let operations = $state(0);
  const lifecycle = (window as any).__appTabLifecycle ??= { mounted: 0, destroyed: 0 };
  $effect(() => {
    recordSupportEvent("test.tab", active ? "active" : "inactive");
  });
  onMount(() => { lifecycle.mounted += 1; });
  onDestroy(() => { lifecycle.destroyed += 1; });
</script>

<section data-testid="tab-probe" data-active={active} data-section={requestedSection} data-highlight={highlightItemId}>
  <output data-testid="operation-count">{operations}</output>
  <button data-testid="run-operation" onclick={() => operations += 1}>Run operation</button>
</section>

<script lang="ts">
  import PluginRenderer from "$lib/components/PluginRenderer.svelte";
  import type { WidgetNode } from "$lib/services/pluginClient";
  let { root }: { root: WidgetNode } = $props();
</script>

<PluginRenderer pluginId="fixture" initialRoot={root}>
  {#snippet header(actions)}
    <header>{@render actions()}</header>
  {/snippet}
</PluginRenderer>

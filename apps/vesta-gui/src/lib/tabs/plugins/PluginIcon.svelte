<script lang="ts">
  import type { PluginManifest } from "$lib/services/pluginClient";
  import { getPluginImageSrc } from "./pluginHelpers";

  interface Props {
    manifest?: PluginManifest;
    iconUrl?: string;
    sizeClass?: string;
  }

  let { manifest, iconUrl, sizeClass = "w-6 h-6" }: Props = $props();
  let imageSrc = $derived(getPluginImageSrc(manifest, iconUrl));
  let iconKind = $derived(manifest?.tab?.icon || manifest?.icon);
</script>

{#if imageSrc}
  <img src={imageSrc} alt="" class="w-full h-full object-contain rounded-lg" />
{:else if iconKind === "film"}
  <svg class={sizeClass} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
{:else if iconKind === "search"}
  <svg class={sizeClass} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
{:else}
  <svg class={sizeClass} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
{/if}

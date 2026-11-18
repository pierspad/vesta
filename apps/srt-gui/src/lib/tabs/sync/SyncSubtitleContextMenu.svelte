<script lang="ts">
  import { locale } from "$lib/i18n";
  import { contextMenu as fitContextMenu } from "$lib/utils/contextMenu";
  import type { SubtitleInfo } from "./syncFormatters";

  interface Props {
    contextMenu: { x: number; y: number; sub: SubtitleInfo };
    syncEditBusy: boolean;
    onClose: () => void;
    onRemoveAnchor: (id: number) => void;
    onPlay: (sub: SubtitleInfo) => void;
    onGoTo: (sub: SubtitleInfo) => void;
  }

  let {
    contextMenu,
    syncEditBusy,
    onClose,
    onRemoveAnchor,
    onPlay,
    onGoTo,
  }: Props = $props();

  let t = $derived($locale);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50"
  onclick={onClose}
  oncontextmenu={(e) => { e.preventDefault(); onClose(); }}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
  role="presentation"
  tabindex="-1"
>
  <div
    use:fitContextMenu={{ x: contextMenu.x, y: contextMenu.y, close: onClose }}
    class="vesta-context-menu animate-fade-in"
  >
    {#if contextMenu.sub.is_anchor}
      <button
        class="vesta-context-menu-item"
        disabled={syncEditBusy}
        onclick={() => {
          const id = contextMenu.sub.id;
          onClose();
          onRemoveAnchor(id);
        }}
      >
        {t("sync.tooltipRemoveAnchor")}
      </button>
    {/if}
    <button
      onclick={() => { onPlay(contextMenu.sub); onClose(); }}
      class="vesta-context-menu-item"
    >
      <span class="inline-flex items-center gap-2">
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
        {t("sync.playSubtitle")}
      </span>
      <kbd>P</kbd>
    </button>
    <button
      onclick={() => { onGoTo(contextMenu.sub); onClose(); }}
      class="vesta-context-menu-item"
    >
      <span class="inline-flex items-center gap-2">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        {t("sync.goToSubtitle")}
      </span>
      <kbd>G</kbd>
    </button>
  </div>
</div>

<script lang="ts">
  import { locale } from "./i18n";

  interface Props {
    x: number;
    y: number;
    hasMedia: boolean;
    onEdit: () => void;
    onMediaSettings: () => void;
    onRemove: () => void;
    onClose: () => void;
  }
  let { x, y, hasMedia, onEdit, onMediaSettings, onRemove, onClose }: Props = $props();

  let t = $derived($locale);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50"
  onclick={onClose}
  oncontextmenu={(e) => {
    e.preventDefault();
    onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
  role="presentation"
  tabindex="-1"
>
  <div class="vesta-context-menu animate-fade-in" style="left: {x}px; top: {y}px;">
    <button type="button" class="vesta-context-menu-item" onclick={onEdit}>
      <span class="inline-flex items-center gap-2">
        <svg class="h-4 w-4 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
        {t("common.edit")}
      </span>
      <kbd>E</kbd>
    </button>
    <button
      type="button"
      class="vesta-context-menu-item"
      disabled={!hasMedia}
      onclick={onMediaSettings}
    >
      <span class="inline-flex items-center gap-2">
        <svg class="h-4 w-4 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        {t("common.settings")}
      </span>
      <kbd>S</kbd>
    </button>
    <div class="vesta-context-menu-separator"></div>
    <button type="button" class="vesta-context-menu-item" onclick={onRemove}>
      <span class="inline-flex items-center gap-2 text-red-300">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
        {t("common.delete")}
      </span>
      <kbd>{t("keys.delShortcut")}</kbd>
    </button>
  </div>
</div>

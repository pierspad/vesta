<script lang="ts">
  import { locale } from "$lib/i18n";
  let { ratio = $bindable<number | null>(null), width, height }: { ratio?: number | null; width: number; height: number } = $props();
  let t = $derived($locale);
</script>
<div class="aspect-link" class:linked={ratio !== null}>
  <span class="connector" aria-hidden="true"></span>
  <button type="button" aria-label={t("flashcards.keepAspectRatio")} title={t("flashcards.keepAspectRatio")} aria-pressed={ratio !== null}
    disabled={!(width > 0 && height > 0)}
    onclick={() => ratio = ratio === null ? width / height : null}
    class="lock-button">
    <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5" y="10" width="14" height="11" rx="2" />
      <path d={ratio !== null ? "M8 10V6a4 4 0 018 0v4" : "M8 10V6a4 4 0 018 0"} /><path d="M12 14v3" />
    </svg>
  </button>
  <span class="connector" aria-hidden="true"></span>
</div>
<style>
  .aspect-link { display: flex; height: 44px; align-items: center; color: var(--color-gray-400); }
  .aspect-link.linked { color: var(--control-focus-border); }
  .connector { width: 8px; border-block-start: 1px dashed var(--control-border); }
  .linked .connector { border-block-start: 2px solid currentColor; }
  .lock-button { display: flex; width: 32px; height: 32px; align-items: center; justify-content: center; border: 1px solid var(--control-border); border-radius: 8px; background: var(--control-bg); color: inherit; cursor: pointer; }
  .linked .lock-button { border-color: var(--control-focus-border); background: var(--control-option-selected); }
  .lock-button:hover { background: var(--control-option-hover); }
  .lock-button:disabled { opacity: .4; cursor: default; }
</style>

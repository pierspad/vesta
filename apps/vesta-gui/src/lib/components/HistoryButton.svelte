<script lang="ts">
  import { locale } from "$lib/i18n";
  let { direction, disabled, onclick }: {
    direction: "undo" | "redo";
    disabled: boolean;
    onclick: () => void;
  } = $props();
  const label = $derived($locale(direction === "undo" ? "common.undo" : "common.redo"));
</script>

<button type="button" class="btn-icon history-button" {disabled} {onclick} title={label} aria-label={label}>
  <svg aria-hidden="true" class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d={direction === "undo" ? "M9 5 4 10l5 5M4 10h10a6 6 0 0 1 6 6v2" : "m15 5 5 5-5 5m5-5H10a6 6 0 0 0-6 6v2"} />
  </svg>
</button>

<style>
  .history-button {
    width: 2.25rem;
    height: 2.25rem;
  }

  .history-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>

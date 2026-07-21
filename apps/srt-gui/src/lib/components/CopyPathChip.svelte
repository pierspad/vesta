<script lang="ts">
  import { getFileName } from "$lib/utils/models";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";

  /**
   * Click-to-copy chip for a result file path, shown in the "finished"
   * state of TranscribeTab/TranslateTab. Both tabs used to redeclare an
   * identical div+SVG block that only differed in which `t()` key backed
   * the two labels -- this component takes the already-translated strings
   * as props so it stays i18n-agnostic. See [[vesta-gui-lib-reorg]].
   */
  interface Props {
    path: string;
    copiedMessage: string;
    tooltip: string;
  }

  let { path, copiedMessage, tooltip }: Props = $props();

  function handleClick() {
    navigator.clipboard.writeText(path);
    snackbar.show(copiedMessage, "success");
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  onclick={handleClick}
  class="flex items-center gap-1.5 text-xs text-gray-400 mt-0.5 cursor-pointer hover:text-white transition-colors bg-white/5 hover:bg-white/10 px-2 py-0.5 rounded border border-white/5 select-all"
  title={tooltip}
>
  <span class="truncate max-w-sm">📁 {getFileName(path)}</span>
  <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
  </svg>
</div>

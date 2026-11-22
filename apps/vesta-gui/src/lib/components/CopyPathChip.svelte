<script lang="ts">
  import { writeClipboard } from "$lib/utils/clipboard";
  import { getFileName } from "$lib/utils/models";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { invokeCommand } from "$lib/services/tauriClient";
  import { locale } from "$lib/i18n";
  import MediaIcon from "./MediaIcon.svelte";
  import { errorMessage } from "$lib/utils/errorMessage";
  let { path, copiedMessage, tooltip }: { path: string; copiedMessage: string; tooltip: string } = $props();
  let t = $derived($locale);
  let opening = $state(false);
  async function copyPath() {
    await writeClipboard(path, copiedMessage);
  }
  async function openFolder() {
    opening = true;
    try { await invokeCommand("open_output_path", { path, folder: true }); }
    catch (error) { snackbar.show(errorMessage(error, t("common.loadFailed")), "error"); }
    finally { opening = false; }
  }
</script>
<div class="flex min-w-0 items-center gap-1.5 mt-0.5 text-xs text-gray-400">
  <button type="button" onclick={copyPath} title={tooltip} aria-label={`${tooltip}: ${path}`} class="flex min-w-0 items-center gap-1.5 hover:text-white transition-colors bg-white/5 hover:bg-white/10 px-2 py-0.5 rounded border border-white/5">
    <MediaIcon kind="file" /><span class="truncate max-w-sm">{getFileName(path)}</span>
    <svg aria-hidden="true" class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 9h11v11H9zM5 15H4V4h11v1" /></svg>
  </button>
  <button type="button" onclick={openFolder} disabled={opening} title={t("common.openOutputFolder")} aria-label={t("common.openOutputFolder")} class="shrink-0 rounded p-1 hover:bg-white/10 hover:text-white disabled:opacity-50"><MediaIcon kind="folder" /></button>
</div>

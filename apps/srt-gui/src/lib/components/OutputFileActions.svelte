<script lang="ts">
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { locale } from "$lib/i18n";
  import { writeClipboard } from "$lib/utils/clipboard";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import MediaIcon from "./MediaIcon.svelte";
  let { path, packageFile = false }: { path: string; packageFile?: boolean } = $props();
  let t = $derived($locale);
  let error = $state("");
  let opening = $state(false);
  async function open(folder: boolean) {
    const selectedPath = path;
    opening = true; error = "";
    try { await invoke("open_output_path", { path: selectedPath, folder }); }
    catch (e) { if (path === selectedPath) error = errorMessage(e, t("common.loadFailed")); }
    finally { opening = false; }
  }
  $effect(() => { path; error = ""; });
</script>
<div class="mt-0.5 min-w-0">
  <div class="flex min-w-0 items-center gap-2 text-[11px] text-gray-400">
    <button type="button" onclick={() => writeClipboard(path, t("common.pathCopied"))} class="flex min-w-0 items-center gap-1 text-left hover:text-gray-200 hover:underline disabled:opacity-50" title={`${t("supportLogs.copyPath")}: ${path}`}><MediaIcon kind={packageFile ? "package" : "file"} /><span class="truncate">{getFileName(path)}</span></button>
    <button type="button" disabled={opening} onclick={() => open(true)} class="shrink-0 rounded p-1 hover:bg-white/10 hover:text-white disabled:opacity-50" title={t("common.openOutputFolder")} aria-label={t("common.openOutputFolder")}><MediaIcon kind="folder" /></button>
    <button type="button" disabled={opening} onclick={() => open(false)} class="shrink-0 rounded p-1 text-cyan-300 hover:bg-white/10 hover:text-cyan-100 disabled:opacity-50" title={`${t("common.openOutputFile")}: ${path}`} aria-label={t("common.openOutputFile")}>
      {#if packageFile && path.toLowerCase().endsWith(".apkg")}
        <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="4"/><path fill="currentColor" stroke="none" d="m12 5 1.6 4.8h5L14.5 13l1.6 4.8-4.1-3-4.1 3 1.6-4.8-4.1-3.2h5Z"/></svg>
      {:else}<MediaIcon kind="file" />{/if}
    </button>
  </div>
  {#if error}<p class="mt-1 max-w-md break-words text-xs text-red-300" role="alert">{error}</p>{/if}
</div>

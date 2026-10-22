<script lang="ts">
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { locale } from "$lib/i18n";
  import { writeClipboard } from "$lib/utils/clipboard";
  import { getFileName } from "$lib/utils/models";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { viewportTooltip } from "$lib/utils/viewportTooltip";
  import MediaIcon from "./MediaIcon.svelte";
  let { path, packageFile = false }: { path: string; packageFile?: boolean } = $props();
  let t = $derived($locale);
  const packageApplication = "Anki";
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
  <div class="flex min-w-0 flex-wrap items-center gap-2 text-[11px] text-gray-300">
    <span class="relative min-w-0 max-w-full">
      <button type="button" onclick={() => writeClipboard(path, t("common.pathCopied"))} class="output-action max-w-full" aria-label={`${t("supportLogs.copyPath")}: ${path}`}><MediaIcon kind={packageFile ? "package" : "file"} /><span class="truncate">{getFileName(path)}</span><svg aria-hidden="true" class="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V4H4v12h4"/></svg></button>
      <span use:viewportTooltip class="output-tooltip">{t("supportLogs.copyPath")}: {path}</span>
    </span>
    <span class="relative">
      <button type="button" disabled={opening} onclick={() => open(true)} class="output-action" aria-label={t("common.openOutputFolder")}><MediaIcon kind="folder" /><span>{t("common.outputFolderLabel")}</span></button>
      <span use:viewportTooltip class="output-tooltip">{t("common.openOutputFolder")}</span>
    </span>
    <span class="relative">
      <button type="button" disabled={opening} onclick={() => open(false)} class="output-action" aria-label={`${t("common.openOutputFile")}: ${path}`}>
        {#if packageFile && path.toLowerCase().endsWith(".apkg")}
          <svg aria-hidden="true" class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="4"/><path fill="currentColor" stroke="none" d="m12 5 1.6 4.8h5L14.5 13l1.6 4.8-4.1-3-4.1 3 1.6-4.8-4.1-3.2h5Z"/></svg><span>{packageApplication}</span>
        {:else}<MediaIcon kind="file" /><span>{t("common.outputFileLabel")}</span>{/if}
      </button>
      <span use:viewportTooltip class="output-tooltip">{t("common.openOutputFile")}: {getFileName(path)}</span>
    </span>
  </div>
  {#if error}<p class="mt-1 max-w-md break-words text-xs text-red-300" role="alert">{error}</p>{/if}
</div>

<style>
  .output-action { display: inline-flex; align-items: center; gap: 6px; min-height: 30px; padding: 5px 8px; border: 1px solid var(--control-border); border-radius: 7px; background: var(--control-bg); color: inherit; cursor: pointer; text-align: start; }
  .output-action:hover { background: var(--control-option-hover); border-color: var(--control-focus-border); }
  .output-action:focus-visible { outline: 2px solid var(--control-focus-border); outline-offset: 2px; }
  .output-action:disabled { opacity: 0.4; cursor: default; }
  .output-tooltip { width: max-content; padding: 8px 12px; border-radius: 8px; background: var(--control-menu-bg); border: 1px solid var(--control-border); color: inherit; font-size: 12px; }
</style>

<script lang="ts">
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import { invokeCommand } from "$lib/services/tauriClient";
  import MediaIcon from "$lib/components/MediaIcon.svelte";
  import { writeClipboard } from "$lib/utils/clipboard";
  import { locale } from "$lib/i18n";
  import { guardedSave } from "$lib/utils/dialogGuard";
  import { supportLogStore as logs } from "$lib/stores/supportLogStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  let t = $derived($locale);
  let opening = $state(false);
  async function openFolder() {
    if (!logs.path || opening) return;
    opening = true;
    try { await invokeCommand("open_output_path", { path: logs.path, folder: true }); }
    catch (error) { logs.error = String(error); }
    finally { opening = false; }
  }
  async function exportLogs() {
    try {
    const path = await guardedSave({ defaultPath: "vesta-support.jsonl", filters: [{ name: "JSON Lines", extensions: ["jsonl"] }] });
    if (!path) return;
    const saved = await logs.export(path);
    if (saved) snackbar.show(`${t("supportLogs.saved")}: ${saved}`, "success");
    } catch (error) { logs.error = String(error); }
  }
</script>
<section class="glass-card p-5">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h3 class="text-lg font-semibold text-rose-300 flex items-center gap-2">
      <svg aria-hidden="true" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M9 12h6m-6 4h6M7 21h10a2 2 0 002-2V9l-6-6H7a2 2 0 00-2 2v14a2 2 0 002 2zm6-18v6h6" /></svg>
      {t("supportLogs.title")}
    </h3>
    <div class="flex gap-2">
      <button type="button" disabled={logs.busy} onclick={() => logs.recording ? logs.stop() : logs.start()} class="flex items-center gap-2 rounded-lg border border-rose-400/30 bg-rose-500/15 px-3 py-2 text-xs font-semibold text-rose-200 transition-colors hover:bg-rose-500/25 focus-visible:outline-2 focus-visible:outline-rose-300 disabled:opacity-40">
        <span class="w-2.5 h-2.5 bg-rose-500 {logs.recording ? 'rounded-sm' : 'rounded-full'}"></span>
        {t(logs.recording ? "supportLogs.stop" : "supportLogs.start")}
      </button>
      <button type="button" disabled={!logs.path || logs.busy} onclick={exportLogs} class="btn-secondary px-3 py-2 text-xs disabled:opacity-40">
        {t("supportLogs.export")}
      </button>
    </div>
  </div>
  <p class="mt-3 min-h-4 text-xs text-rose-300 flex items-center gap-2" class:invisible={!logs.recording} role="status"><span class="h-2 w-2 rounded-full bg-rose-500 animate-pulse"></span>{logs.recording ? t("supportLogs.recording") : ""}</p>
    <div class="mt-4">
      <label for="support-log-path" class="mb-2 block text-xs font-medium text-gray-400">{t("supportLogs.path")}</label>
      <div class="flex items-center gap-2">
      <input id="support-log-path" readonly value={logs.path ?? ""} class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/20 px-3 py-2 font-mono text-xs text-gray-200" />
      <button type="button" onclick={openFolder} disabled={opening || !logs.path} title={t("supportLogs.openFolder")} aria-label={t("supportLogs.openFolder")} class="btn-secondary shrink-0 p-2 disabled:opacity-40"><MediaIcon kind="folder" /></button>
      <button type="button" disabled={!logs.path} title={t("supportLogs.copyPath")} aria-label={t("supportLogs.copyPath")} onclick={() => void writeClipboard(logs.path!, t("supportLogs.copied"))} class="btn-secondary p-2 shrink-0 disabled:opacity-40"><svg aria-hidden="true" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.8"><path d="M9 9h11v11H9zM5 15H4V4h11v1" /></svg></button>
      </div>
    </div>
    <div class="mt-4">
      <div class="mb-2 flex items-center justify-between gap-2">
        <span class="text-xs text-gray-400">{logs.preview.length}/200</span>
        <button type="button" disabled={!logs.preview.length} onclick={() => void writeClipboard(logs.previewText, t("supportLogs.copied"))} class="btn-secondary p-2 disabled:opacity-40" aria-label={t("common.copy")} title={t("common.copy")}>
          <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M9 9h11v11H9zM5 15H4V4h11v1" /></svg>
        </button>
      </div>
      <div class="relative">
        {#if !logs.preview.length}<EmptyStatusLabel message={t("supportLogs.empty")} />{/if}
      <textarea readonly value={logs.previewText} aria-label={t("supportLogs.title")} spellcheck="false" class="log-preview block h-48 w-full resize-y rounded-lg border border-white/10 bg-black/25 p-3 font-mono text-xs leading-relaxed text-gray-200 focus:border-rose-400/50 focus:outline-none"></textarea>
      </div>
    </div>
  {#if logs.error}<p class="mt-3 text-xs text-red-300 break-all" role="alert">{logs.error}</p>{/if}
</section>

<style>
  .log-preview {
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
    scrollbar-gutter: stable;
    color-scheme: dark;
  }
  .log-preview::-webkit-scrollbar { width: 8px; height: 8px; }
  .log-preview::-webkit-scrollbar-track, .log-preview::-webkit-scrollbar-corner { background: transparent; }
  .log-preview::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 4px; }
  .log-preview::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.3); }
</style>

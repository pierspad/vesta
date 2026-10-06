<script lang="ts">
  import { locale } from "$lib/i18n";
  import { guardedSave } from "$lib/utils/dialogGuard";
  import { supportLogStore as logs } from "$lib/stores/supportLogStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  let t = $derived($locale);
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
      <button type="button" disabled={logs.busy} onclick={() => logs.recording ? logs.stop() : logs.start()} class="btn-secondary flex items-center gap-2 px-3 py-2 text-xs disabled:opacity-40">
        <span class="w-2.5 h-2.5 bg-rose-500 {logs.recording ? 'rounded-sm' : 'rounded-full'}"></span>
        {t(logs.recording ? "supportLogs.stop" : "supportLogs.start")}
      </button>
      <button type="button" disabled={!logs.path || logs.busy} onclick={exportLogs} class="btn-secondary px-3 py-2 text-xs disabled:opacity-40">
        {t("supportLogs.export")}
      </button>
    </div>
  </div>
  <p class="mt-3 text-xs text-gray-400 leading-relaxed">{t("supportLogs.description")}</p>
  {#if logs.recording}<p class="mt-3 text-xs text-rose-300 flex items-center gap-2" role="status"><span class="h-2 w-2 rounded-full bg-rose-500 animate-pulse"></span>{t("supportLogs.recording")}</p>{/if}
  {#if logs.path}
    <div class="mt-3 flex items-center gap-2 rounded-lg border border-white/10 bg-black/20 p-3">
      <div class="min-w-0 flex-1"><span class="text-[10px] text-gray-400">{t("supportLogs.path")}</span><p class="text-xs text-gray-200 break-all select-text font-mono">{logs.path}</p></div>
      <button type="button" title={t("supportLogs.copyPath")} aria-label={t("supportLogs.copyPath")} onclick={async () => { try { await navigator.clipboard.writeText(logs.path!); snackbar.show(t("supportLogs.copied"), "success"); } catch (error) { logs.error = String(error); } }} class="btn-secondary p-2 shrink-0"><svg aria-hidden="true" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.8"><path d="M9 9h11v11H9zM5 15H4V4h11v1" /></svg></button>
    </div>
  {/if}
  {#if logs.error}<p class="mt-3 text-xs text-red-300 break-all" role="alert">{logs.error}</p>{/if}
</section>

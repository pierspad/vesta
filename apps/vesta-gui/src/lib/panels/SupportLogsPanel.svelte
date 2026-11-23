<script lang="ts">
  import { onDestroy } from "svelte";
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
  let copiedLogs = $state(false);
  let copiedPath = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  let copyPathTimer: ReturnType<typeof setTimeout> | null = null;
  onDestroy(() => {
    if (copyTimer) clearTimeout(copyTimer);
    if (copyPathTimer) clearTimeout(copyPathTimer);
  });
  async function handleCopyPath() {
    if (!logs.path) return;
    if (await writeClipboard(logs.path, t("supportLogs.copied"))) {
      copiedPath = true;
      if (copyPathTimer) clearTimeout(copyPathTimer);
      copyPathTimer = setTimeout(() => {
        copiedPath = false;
      }, 2000);
    }
  }
  async function handleCopyLogs() {
    if (!logs.preview.length) return;
    if (await writeClipboard(logs.previewText, t("common.copied"))) {
      copiedLogs = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copiedLogs = false;
      }, 2000);
    }
  }
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
      <button type="button" disabled={!logs.preview.length} onclick={() => logs.clear()} class="btn-secondary px-3 py-2 text-xs disabled:opacity-40">
        {t("shortcuts.action.clearLogs")}
      </button>
    </div>
  </div>
  <p class="mt-3 text-xs text-gray-400">{t("supportLogs.description")}</p>
  <p class="mt-3 min-h-4 text-xs text-rose-300 flex items-center gap-2" class:invisible={!logs.recording} role="status"><span class="h-2 w-2 rounded-full bg-rose-500 animate-pulse"></span>{logs.recording ? t("supportLogs.recording") : ""}</p>
    <div class="mt-4">
      <label for="support-log-path" class="mb-2 block text-xs font-medium text-gray-400">{t("supportLogs.path")}</label>
      <div class="flex items-center gap-2">
        <div class="relative min-w-0 flex-1">
          <input id="support-log-path" readonly value={logs.path ?? ""} class="w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 pr-10 font-mono text-xs text-gray-200" />
          <button
            type="button"
            disabled={!logs.path}
            title={t("supportLogs.copyPath")}
            aria-label={t("supportLogs.copyPath")}
            onclick={handleCopyPath}
            class="absolute right-1.5 top-1/2 -translate-y-1/2 w-7 h-7 rounded-md bg-gray-900/80 hover:bg-cyan-500/20 text-gray-400 hover:text-cyan-200 border border-white/10 hover:border-cyan-500/30 transition-all flex items-center justify-center cursor-pointer opacity-75 hover:opacity-100 disabled:opacity-30 disabled:pointer-events-none"
          >
            {#if copiedPath}
              <svg aria-hidden="true" class="w-3.5 h-3.5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
            {:else}
              <svg aria-hidden="true" class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.8">
                <path d="M9 9h11v11H9zM5 15H4V4h11v1" />
              </svg>
            {/if}
          </button>
        </div>
        <button
          type="button"
          onclick={openFolder}
          disabled={opening || !logs.path}
          title={t("supportLogs.openFolder")}
          aria-label={t("supportLogs.openFolder")}
          class="btn-secondary shrink-0 inline-flex items-center gap-1.5 px-3 py-2 text-xs disabled:opacity-40"
        >
          <MediaIcon kind="folder" />
          <span>{t("supportLogs.openFolder")}</span>
        </button>
      </div>
    </div>
    <div class="mt-4">
      <div class="mb-2 flex items-center justify-between gap-2">
        <span class="text-xs text-gray-400">{logs.preview.length}/200</span>
      </div>
      <div class="relative">
        {#if !logs.preview.length}<EmptyStatusLabel message={t("supportLogs.empty")} />{/if}
        <textarea readonly value={logs.previewText} aria-label={t("supportLogs.title")} spellcheck="false" class="log-preview block h-48 w-full resize-y rounded-lg border border-white/10 bg-black/25 p-3 pr-12 font-mono text-xs leading-relaxed text-gray-200 focus:border-rose-400/50 focus:outline-none"></textarea>
        <button
          type="button"
          disabled={!logs.preview.length}
          onclick={handleCopyLogs}
          class="absolute top-2.5 right-2.5 z-10 w-8 h-8 rounded-lg bg-gray-900/80 hover:bg-cyan-500/20 text-gray-400 hover:text-cyan-200 border border-white/10 hover:border-cyan-500/30 transition-all flex items-center justify-center cursor-pointer shadow-md opacity-60 hover:opacity-100 disabled:opacity-30 disabled:pointer-events-none"
          aria-label={t("common.copy")}
          title={t("common.copy")}
        >
          {#if copiedLogs}
            <svg aria-hidden="true" class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
          {:else}
            <svg aria-hidden="true" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
          {/if}
        </button>
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

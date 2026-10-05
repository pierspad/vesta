<script lang="ts">
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import LoadingSpinner from "$lib/components/LoadingSpinner.svelte";
  import { locale } from "$lib/i18n";
  let { title, text, loading, saving, error, onclose, hasPrevious, hasNext, onprevious, onnext, onselect, ondownload } = $props<{ title: string; text: string; loading: boolean; saving: boolean; error: string; hasPrevious: boolean; hasNext: boolean; onprevious: () => void; onnext: () => void; onselect: () => void; ondownload: () => void; onclose: () => void }>();
  let t = $derived($locale);
  let dialog: HTMLDialogElement;
  $effect(() => { dialog.showModal(); });
</script>
<dialog bind:this={dialog} oncancel={(event) => { event.preventDefault(); onclose(); }} onkeydown={(event) => { if (event.target instanceof HTMLTextAreaElement || saving || event.altKey || event.ctrlKey || event.metaKey) return; if (event.key === "ArrowLeft" && hasPrevious) { event.preventDefault(); onprevious(); } if (event.key === "ArrowRight" && hasNext) { event.preventDefault(); onnext(); } }} aria-label={title} class="m-auto w-[min(850px,90vw)] rounded-2xl border border-white/15 bg-gray-900 p-6 text-gray-100 shadow-2xl backdrop:bg-black/70">
  <div class="mb-4 flex items-center justify-between gap-4"><h2 class="text-lg font-semibold text-teal-300">{title}</h2><button class="btn-secondary preview-button close-button" onclick={onclose} aria-label={t("common.cancel")}><svg aria-hidden="true" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m6 6 12 12M18 6 6 18" /></svg></button></div>
  <div class="h-[min(60vh,600px)]" aria-busy={loading}>
    {#if loading}<div class="flex h-full items-center justify-center" role="status" aria-label={t("extract.working")}><LoadingSpinner /></div>
    {:else if error}<p role="alert" class="text-red-300">{error}</p>
    {:else}{#key title}<CodeEditor value={text} language="text" readonly heightClass="h-full" />{/key}{/if}
  </div>
  <div class="mt-4 flex flex-wrap items-center gap-2">
    <button type="button" class="btn-secondary preview-button" disabled={saving || !hasPrevious} onclick={onprevious}>
      <svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m15 6-6 6 6 6" /></svg>{t("extract.previous")}
    </button>
    <button type="button" class="btn-secondary preview-button" disabled={saving || !hasNext} onclick={onnext}>
      {t("extract.next")}<svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 6 6 6-6 6" /></svg>
    </button>
    <div class="ml-auto flex flex-wrap gap-2">
      <button type="button" class="btn-secondary preview-button" disabled={loading || saving || !!error} onclick={onselect}>
        <svg aria-hidden="true" class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6" /></svg>{t("extract.select")}
      </button>
      <button type="button" class="preview-button save-button" disabled={loading || saving || !!error} onclick={ondownload}>
        <svg aria-hidden="true" class="h-4 w-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m-5-5 5 5 5-5M5 16v4h14v-4" /></svg>{t("extract.save")}
      </button>
    </div>
  </div>
  <p class="mt-3 text-xs text-gray-400">{t("extract.previewLimit")}</p>
</dialog>

<style>
  /* Scoped specificity overrides the global button padding, which beats utilities. */
  .preview-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    min-height: 2.25rem;
    padding: 0.375rem 0.75rem;
    border-radius: 0.5rem;
    font-size: 0.75rem;
    line-height: 1rem;
    color: #f3f4f6;
  }
  .preview-button:disabled { opacity: 0.35; cursor: not-allowed; }
  .save-button { background: rgb(16 185 129 / 0.15); border: 1px solid rgb(52 211 153 / 0.4); color: #6ee7b7; }
  .save-button:hover:not(:disabled) { background: rgb(16 185 129 / 0.25); }
  .close-button { width: 2.25rem; padding: 0; flex-shrink: 0; }
</style>

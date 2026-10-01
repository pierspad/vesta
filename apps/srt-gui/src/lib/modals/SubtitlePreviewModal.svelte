<script lang="ts">
  import { locale } from "$lib/i18n";
  let { title, text, loading, error, onclose } = $props<{ title: string; text: string; loading: boolean; error: string; onclose: () => void }>();
  let t = $derived($locale);
  let dialog: HTMLDialogElement;
  $effect(() => { dialog.showModal(); });
</script>
<dialog bind:this={dialog} oncancel={(event) => { event.preventDefault(); onclose(); }} aria-label={title} class="m-auto w-[min(850px,90vw)] rounded-2xl border border-white/15 bg-gray-900 p-6 text-gray-100 shadow-2xl backdrop:bg-black/70">
  <div class="mb-4 flex items-center justify-between gap-4"><h2 class="text-lg font-semibold text-teal-300">{title}</h2><button class="btn-secondary px-3 py-1" onclick={onclose} aria-label={t("common.cancel")}>✕</button></div>
  <div class="h-[min(60vh,600px)] overflow-y-auto rounded-xl border border-white/10 bg-black/20 p-4 scrollbar-thin" aria-busy={loading}>
    {#if loading}<p role="status" class="text-teal-300">{t("extract.working")}</p>
    {:else if error}<p role="alert" class="text-red-300">{error}</p>
    {:else}<pre class="whitespace-pre-wrap break-words font-mono text-sm leading-relaxed">{text}</pre>{/if}
  </div>
  <p class="mt-3 text-xs text-gray-400">{t("extract.previewLimit")}</p>
</dialog>

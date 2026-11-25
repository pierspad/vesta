<script lang="ts">
  import { locale } from "$lib/i18n";
  import PathPickerField from "$lib/components/PathPickerField.svelte";

  interface Props {
    filePath: string;
    totalCards: number;
    annotatedCount: number;
    modifiedCount: number;
    disabled: boolean;
    highlightClass?: string;
    onBrowse: () => void;
  }

  let { filePath, totalCards, annotatedCount, modifiedCount, disabled, highlightClass = "", onBrowse }: Props = $props();
  let t = $derived($locale);
</script>

<div class="glass-card p-5 shrink-0 {highlightClass}">
  <h3 class="panel-title-files-output mb-4 flex items-center gap-2 text-lg font-semibold">
    <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
    {t("common.filesAndOutput")}
  </h3>

  <div class="grid grid-cols-1 md:grid-cols-2 gap-4 md:items-end">
    <PathPickerField
      label={t("refine.deckLabel")}
      labelIcon="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
      required
      value={filePath || ""}
      placeholder={t("refine.deckPlaceholder")}
      browseTitle={t("refine.dropzone.browse")}
      onbrowse={onBrowse}
      {disabled}
    />

    <div class="grid grid-cols-3 gap-3">
      <div class="h-[42px] bg-white/5 border border-white/10 rounded-xl px-2 py-1 text-center flex flex-col justify-center">
        <span class="block text-[9px] font-bold text-gray-500 uppercase tracking-wider truncate leading-tight">{t("refine.stats.totalCards")}</span>
        <span class="text-xs font-bold text-gray-200 leading-tight">{totalCards}</span>
      </div>
      <div class="h-[42px] bg-white/5 border border-white/10 rounded-xl px-2 py-1 text-center flex flex-col justify-center">
        <span class="block text-[9px] font-bold text-gray-500 uppercase tracking-wider truncate leading-tight">{t("refine.stats.annotated")}</span>
        <span class="text-xs font-bold text-rose-300 leading-tight">{annotatedCount}</span>
      </div>
      <div class="h-[42px] bg-white/5 border border-white/10 rounded-xl px-2 py-1 text-center flex flex-col justify-center">
        <span class="block text-[9px] font-bold text-gray-500 uppercase tracking-wider truncate leading-tight">{t("refine.stats.modified")}</span>
        <span class="text-xs font-bold text-amber-300 leading-tight">{modifiedCount}</span>
      </div>
    </div>
  </div>
</div>

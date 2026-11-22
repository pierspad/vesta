<script lang="ts">
  import { locale } from "$lib/i18n";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import type { SrtFileInfo } from "$lib/services/translate";

  interface Props {
    inputPath: string;
    outputPath: string;
    fileInfo: SrtFileInfo | null;
    canUseFilePanel: boolean;
    locked: boolean;
    highlightClass: string;
    onSelectInput: () => void;
    onSelectOutput: () => void;
    onExpand: (field: "input" | "output") => void;
  }

  let {
    inputPath,
    outputPath,
    fileInfo,
    canUseFilePanel,
    locked,
    highlightClass,
    onSelectInput,
    onSelectOutput,
    onExpand,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div
  inert={!canUseFilePanel || locked}
  title={!canUseFilePanel ? t("translate.selectProviderFirst") : undefined}
  class="glass-card p-5 shrink-0 {highlightClass} {!canUseFilePanel ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-4">
    <h3 class="text-lg font-semibold flex items-center gap-2 panel-title-files-output">
      <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
      </svg>
      {t("common.filesAndOutput")}
    </h3>
    {#if fileInfo}
      <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-indigo-500/10 text-indigo-300 border border-indigo-500/20">
        <svg aria-hidden="true" class="w-3.5 h-3.5 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6M7 21h10a2 2 0 002-2V9l-6-6H7a2 2 0 00-2 2v14a2 2 0 002 2zm6-18v6h6" /></svg>
        <span>{fileInfo.subtitle_count} {t("translate.subtitles")}</span>
      </div>
    {/if}
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 gap-4 md:items-end">
    <PathPickerField
      label={t("translate.inputFile")}
      labelIcon="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10"
      value={inputPath}
      placeholder={t("translate.selectFile")}
      browseTitle={t("translate.tooltip.upload")}
      onexpand={() => onExpand("input")}
      onbrowse={onSelectInput}
      required={true}
    />

    <PathPickerField
      label={t("translate.outputFile")}
      labelIcon="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
      value={outputPath}
      placeholder={t("translate.selectDestination")}
      browseTitle={t("translate.tooltip.save")}
      browseLabel={t("translate.saveAs")}
      browseIconPath="M12 4v12m-4-4 4 4 4-4M4 16v4h16v-4"
      onexpand={() => onExpand("output")}
      onbrowse={onSelectOutput}
      required={true}
    />
  </div>
</div>

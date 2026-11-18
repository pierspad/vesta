<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import MediaIcon from "$lib/components/MediaIcon.svelte";

  interface Props {
    inputPath: string;
    outputPath: string;
    disabled: boolean;
    highlightClass: string;
    onSelectInput: () => void;
    onSelectOutput: () => void;
    onExpand: (field: "input" | "output") => void;
  }

  let {
    inputPath,
    outputPath,
    disabled,
    highlightClass,
    onSelectInput,
    onSelectOutput,
    onExpand,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-5 {highlightClass}">
  <h3 class="text-lg font-semibold mb-4 flex items-center gap-2 panel-title-files-output">
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
    </svg>
    {t("common.filesAndOutput")}
  </h3>
  <div class="grid grid-cols-1 gap-4 md:grid-cols-2 md:items-end">
    <PathPickerField
      label={t("transcribe.inputMediaFile")}
      labelIcon="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z"
      value={inputPath}
      placeholder={t("transcribe.noInputMediaSelected")}
      browseTitle={t("transcribe.selectFile")}
      onexpand={() => onExpand("input")}
      {disabled}
      onbrowse={onSelectInput}
      required={true}
    />

    <PathPickerField
      label={t("transcribe.outputSrtFile")}
      labelIcon="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
      value={outputPath}
      placeholder={t("transcribe.noOutputFileSelected")}
      browseTitle={t("transcribe.selectDestination")}
      onexpand={() => onExpand("output")}
      {disabled}
      onbrowse={onSelectOutput}
      required={true}
    />
    {#if inputPath}
      <div class="p-3 bg-cyan-500/10 border border-cyan-500/30 rounded-lg md:col-span-2">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-cyan-500/20 flex items-center justify-center">
            <MediaIcon kind="video" class="w-5 h-5 text-cyan-300" />
          </div>
          <div>
            <p class="font-medium text-white">{getFileName(inputPath)}</p>
            <p class="text-sm text-gray-400">{t("transcribe.readyToProcess")}</p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

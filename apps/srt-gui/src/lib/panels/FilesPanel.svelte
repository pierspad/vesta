<script lang="ts">
  import { locale } from "$lib/i18n";
  import PathPickerField from "$lib/components/PathPickerField.svelte";

  interface Props {
    srtPath: string | null;
    mediaPath: string | null;
    srtLoaded: boolean;
    onExpandSrt: () => void;
    onExpandMedia: () => void;
    onBrowseSrt: () => void;
    onBrowseMedia: () => void;
  }

  let {
    srtPath,
    mediaPath,
    srtLoaded,
    onExpandSrt,
    onExpandMedia,
    onBrowseSrt,
    onBrowseMedia,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-5">
  <h3
    class="text-lg font-semibold mb-4 flex items-center gap-2 panel-title-files-output"
  >
    <svg
      class="w-5 h-5"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
      />
    </svg>
    {t("common.filesAndOutput")}
  </h3>

  <div class="space-y-3">
    <!-- SRT File input -->
    <PathPickerField
      label={t("sync.inputSrtFile")}
      value={srtPath || ""}
      placeholder={t("sync.noSrtFileSelected")}
      browseTitle={t("sync.tooltip.loadSrt")}
      onexpand={() => {
        if (srtPath) onExpandSrt();
      }}
      onbrowse={onBrowseSrt}
      required={true}
    />

    <!-- Media File input -->
    <PathPickerField
      label={t("sync.inputMediaFile")}
      value={mediaPath || ""}
      placeholder={t("sync.noMediaFileSelected")}
      browseTitle={t("sync.tooltip.loadVideo") || ""}
      disabled={!srtLoaded}
      onexpand={() => {
        if (mediaPath) onExpandMedia();
      }}
      onbrowse={onBrowseMedia}
      required={true}
    />
  </div>
</div>

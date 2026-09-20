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
      labelIcon="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10"
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
      labelIcon="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z"
      value={mediaPath || ""}
      placeholder={t("sync.noMediaFileSelected")}
      browseTitle={t("sync.tooltip.loadVideo")}
      disabled={!srtLoaded}
      onexpand={() => {
        if (mediaPath) onExpandMedia();
      }}
      onbrowse={onBrowseMedia}
      required={true}
    />
  </div>
</div>

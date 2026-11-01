<script lang="ts">
  import type { WhisperModel } from "$lib/services/transcribe";
  import { locale } from "$lib/i18n";
  import ProgressFill from "$lib/components/ProgressFill.svelte";

  let {
    models,
    value,
    disabled = false,
    downloadingModelId = null,
    progress = 0,
    isDownloading = false,
    onselect,
    oncancel,
    ondownload,
    oncontextmenu,
    ondblclick,
  }: {
    models: WhisperModel[];
    value: string;
    disabled?: boolean;
    downloadingModelId?: string | null;
    progress?: number;
    isDownloading?: boolean;
    onselect: (model: WhisperModel) => void;
    oncancel?: () => void;
    ondownload?: (id: string) => void;
    oncontextmenu?: (event: MouseEvent, model: WhisperModel) => void;
    ondblclick?: (model: WhisperModel) => void;
  } = $props();
  let t = $derived($locale);

  const iconPath = (modelId: string) => ({
    tiny: "M13 3L4 14h7l-1 7 9-12h-7l1-6z",
    base: "M5 15v2m4-6v6m4-10v10m4-7v7m4-4v4",
    small: "M4 12h3l2-5 4 10 2-5h5",
    medium: "M4 6h16M7 12h10M10 18h4",
    large: "M12 6v6l4 2m5-2a9 9 0 11-18 0 9 9 0 0118 0z",
  })[modelId] ?? "M9 3h6m-7 4h8a3 3 0 013 3v7a3 3 0 01-3 3H8a3 3 0 01-3-3v-7a3 3 0 013-3zm4 3v4m-2-2h4";

  const accent = (modelId: string) => ({
    tiny: "bg-amber-500/15 text-amber-200",
    base: "bg-sky-500/15 text-sky-200",
    small: "bg-emerald-500/15 text-emerald-200",
    medium: "bg-indigo-500/15 text-indigo-200",
    large: "bg-fuchsia-500/15 text-fuchsia-200",
  })[modelId] ?? "bg-cyan-500/15 text-cyan-200";
</script>

<div class="grid grid-cols-5 gap-2" role="radiogroup" aria-label={t("settings.whisper.modelLabel")}>
  {#each models as model (model.id)}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      role="radio"
      aria-checked={value === model.id}
      aria-disabled={disabled}
      tabindex={disabled ? -1 : 0}
      onclick={() => {
        if (!disabled) onselect(model);
      }}
      onkeydown={(e) => {
        if (!disabled && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          onselect(model);
        }
      }}
      oncontextmenu={(event) => oncontextmenu?.(event, model)}
      ondblclick={() => ondblclick?.(model)}
      class="group relative min-w-0 overflow-hidden rounded-xl border px-2 py-3 text-center transition-colors select-none {disabled ? 'cursor-not-allowed opacity-45' : 'cursor-pointer'}
        {value === model.id
          ? 'border-cyan-400/60 bg-cyan-500/15 text-white'
          : 'border-white/10 bg-white/[0.035] text-gray-400 hover:border-white/20 hover:bg-white/[0.07]'}"
    >
      {#if isDownloading && downloadingModelId === model.id}
        <ProgressFill value={progress} />
      {/if}

      <!-- Top-Right Action Button -->
      <div class="absolute top-1.5 right-1.5 z-10 flex items-center">
        {#if !model.downloaded}
          {#if isDownloading && downloadingModelId === model.id}
            <button
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                oncancel?.();
              }}
              class="text-red-400 hover:text-red-300 transition-colors p-1 bg-red-500/15 hover:bg-red-500/25 rounded-md border border-red-500/30 flex items-center justify-center cursor-pointer shadow-sm"
              title={t("common.cancel")}
              aria-label={t("common.cancel")}
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 6h12v12H6z" />
              </svg>
            </button>
          {:else}
            <button
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                if (ondownload) {
                  ondownload(model.id);
                } else {
                  onselect(model);
                }
              }}
              disabled={disabled || isDownloading}
              class="text-amber-400 hover:text-cyan-300 transition-colors p-1 bg-amber-500/10 hover:bg-cyan-500/20 border border-amber-500/20 hover:border-cyan-500/40 rounded-md cursor-pointer shadow-sm disabled:cursor-not-allowed disabled:opacity-40"
              title={t("settings.download")}
              aria-label={t("settings.download")}
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
              </svg>
            </button>
          {/if}
        {/if}
      </div>

      <span class="mx-auto mb-2 flex h-9 w-9 items-center justify-center rounded-lg border border-white/10 {accent(model.id)}">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath(model.id)} />
        </svg>
      </span>
      <span class="block truncate text-xs font-bold">{model.name}</span>
      <span class="mt-1 block truncate text-[10px] text-gray-500">{model.size}</span>
      <span class="mt-1 block truncate text-[9px] font-semibold {model.downloaded ? 'text-emerald-400' : 'text-amber-400/70'}">
        {#if isDownloading && downloadingModelId === model.id}
          {t("settings.downloading")}{progress > 0 ? ` ${progress}%` : ""}
        {:else if model.downloaded}
          {t("settings.ready")}
        {:else}
          {t("settings.notDownloaded")}
        {/if}
      </span>
    </div>
  {/each}
</div>

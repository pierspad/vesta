<script lang="ts">
  import type { WhisperModel } from "$lib/services/transcribe";
  import { locale } from "$lib/i18n";

  let {
    models,
    value,
    disabled = false,
    onselect,
    oncontextmenu,
    ondblclick,
  }: {
    models: WhisperModel[];
    value: string;
    disabled?: boolean;
    onselect: (model: WhisperModel) => void;
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

<div class="grid grid-cols-5 gap-2" role="radiogroup" aria-label="Whisper model">
  {#each models as model (model.id)}
    <button
      type="button"
      role="radio"
      aria-checked={value === model.id}
      {disabled}
      onclick={() => onselect(model)}
      oncontextmenu={(event) => oncontextmenu?.(event, model)}
      ondblclick={() => ondblclick?.(model)}
      class="relative min-w-0 rounded-xl border px-2 py-3 text-center transition-colors disabled:cursor-not-allowed disabled:opacity-45
        {value === model.id
          ? 'border-cyan-400/60 bg-cyan-500/15 text-white'
          : 'border-white/10 bg-white/[0.035] text-gray-400 hover:border-white/20 hover:bg-white/[0.07]'}"
    >
      <span class="mx-auto mb-2 flex h-9 w-9 items-center justify-center rounded-lg border border-white/10 {accent(model.id)}">
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={iconPath(model.id)} />
        </svg>
      </span>
      <span class="block truncate text-xs font-bold">{model.name}</span>
      <span class="mt-1 block truncate text-[10px] text-gray-500">{model.size}</span>
      <span class="mt-1 block truncate text-[9px] font-semibold {model.downloaded ? 'text-emerald-400' : 'text-amber-400/70'}">
        {model.downloaded ? t("settings.ready") : t("settings.notDownloaded")}
      </span>
    </button>
  {/each}
</div>

<script lang="ts">
  import { t } from "$lib/i18n";

  interface Props {
    label?: string;
    value: string;
    placeholder: string;
    browseTitle: string;
    onexpand: () => void;
    onbrowse: () => void;
    disabled?: boolean;
    onclear?: () => void;
    browseButtonClass?: string;
    browseIconPath?: string;
    browseLabel?: string;
    required?: boolean;
  }

  let {
    label = "",
    value,
    placeholder,
    browseTitle,
    onexpand,
    onbrowse,
    disabled = false,
    onclear = undefined,
    browseButtonClass = "btn-secondary py-2 px-3",
    browseIconPath = "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z",
    browseLabel = t("flashcards.browse"),
    required = false,
  }: Props = $props();
</script>

<div>
  {#if label}
    <span class="block text-xs text-gray-400 mb-1">
      {label}
      {#if required}
        <span class="text-red-400 font-bold ml-0.5">*</span>
      {/if}
    </span>
  {/if}
  <div class="flex gap-2">
    <button
      type="button"
      onclick={onexpand}
      {disabled}
      class="input-modern flex-1 text-sm text-left cursor-pointer hover:bg-white/10 transition-colors truncate disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:bg-white/5"
      style="direction: rtl; text-align: left;"
      title={value || placeholder}
    >
      <span
        class={value ? "text-white" : "text-gray-500"}
        style="unicode-bidi: plaintext;"
      >
        {value || placeholder}
      </span>
    </button>

    <button
      type="button"
      onclick={onbrowse}
      {disabled}
      class="{browseButtonClass} flex items-center justify-center gap-1.5 whitespace-nowrap shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
      title={browseTitle}
    >
      <svg
        class="w-4 h-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d={browseIconPath}
        />
      </svg>
      {#if browseLabel}
        <span class="text-xs font-semibold">{browseLabel}</span>
      {/if}
    </button>

    {#if onclear}
      <button
        type="button"
        onclick={onclear}
        disabled={disabled || !value}
        class="inline-flex py-2 px-3 flex-shrink-0 items-center justify-center rounded-lg border transition-colors cursor-pointer disabled:cursor-not-allowed disabled:border-white/10 disabled:bg-white/5 disabled:text-gray-600 disabled:opacity-60 border-red-500/30 bg-red-500/10 text-red-300 hover:border-red-400/60 hover:bg-red-500/20"
        title="Rimuovi file"
        aria-label="Rimuovi file"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
        </svg>
      </button>
    {/if}
  </div>
</div>

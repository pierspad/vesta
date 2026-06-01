<script lang="ts">
  import { t } from "./i18n";

  interface Props {
    label: string;
    value: string;
    placeholder: string;
    browseTitle: string;
    onexpand: () => void;
    onbrowse: () => void;
    browseButtonClass?: string;
    browseIconPath?: string;
    browseLabel?: string;
  }

  let {
    label,
    value,
    placeholder,
    browseTitle,
    onexpand,
    onbrowse,
    browseButtonClass = "btn-secondary py-2 px-3",
    browseIconPath = "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z",
    browseLabel = t("flashcards.browse"),
  }: Props = $props();
</script>

<div>
  <span class="block text-sm text-gray-400 mb-1">{label}</span>
  <div class="flex gap-2">
    <button
      type="button"
      onclick={onexpand}
      class="input-modern flex-1 text-sm text-left cursor-pointer hover:bg-white/10 transition-colors truncate"
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
      class="{browseButtonClass} flex items-center justify-center gap-1.5 whitespace-nowrap shrink-0"
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
  </div>
</div>

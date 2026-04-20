<script lang="ts">
  interface Props {
    label: string;
    value: string;
    placeholder: string;
    browseTitle: string;
    onexpand: () => void;
    onbrowse: () => void;
    browseButtonClass?: string;
    browseIconPath?: string;
  }

  let {
    label,
    value,
    placeholder,
    browseTitle,
    onexpand,
    onbrowse,
    browseButtonClass = "btn-primary py-2 px-3",
    browseIconPath = "M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12",
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
      class={browseButtonClass}
      title={browseTitle}
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
          d={browseIconPath}
        />
      </svg>
    </button>
  </div>
</div>

<script lang="ts">
  interface Props {
    isOpen: boolean;
    title: string;
    value: string;
    onclose: () => void;
    confirmText?: string;
    secondaryText?: string;
    onsecondary?: () => void;
  }

  let {
    isOpen,
    title,
    value,
    onclose,
    confirmText = "OK",
    secondaryText,
    onsecondary,
  }: Props = $props();
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-6"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={onclose}
    onkeydown={(e) => {
      if (e.key === "Escape") onclose();
    }}
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-2xl p-5 animate-fade-in"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-semibold text-gray-300">{title}</h3>
        <button
          type="button"
          onclick={onclose}
          class="text-gray-400 hover:text-white text-lg leading-none">✕</button
        >
      </div>

      <div class="bg-gray-800/80 rounded-lg p-3 border border-gray-700/50">
        <p class="text-sm text-white font-mono break-all select-all leading-relaxed">
          {value || "—"}
        </p>
      </div>

      <div class="mt-3 flex justify-end gap-2">
        {#if secondaryText && onsecondary}
          <button
            type="button"
            onclick={onsecondary}
            class="btn-secondary py-1.5 px-4 text-xs"
          >
            {secondaryText}
          </button>
        {/if}

        <button
          type="button"
          onclick={onclose}
          class="btn-primary py-1.5 px-4 text-xs"
        >
          {confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}

<script lang="ts">
  import { locale } from "$lib/i18n";
  import { generationStore } from "$lib/stores/generationStore.svelte";

  interface Props {
    hasAnyFiles: boolean;
    needsDeckName: boolean;
    highlightClass: string;
    hintLoadTargetFirst: string;
  }

  let { hasAnyFiles, needsDeckName, highlightClass, hintLoadTargetFirst }: Props = $props();

  let t = $derived($locale);
</script>

<div
  inert={!hasAnyFiles}
  title={!hasAnyFiles ? hintLoadTargetFirst : undefined}
  class="glass-card p-5 {highlightClass} {!hasAnyFiles
    ? 'opacity-50'
    : ''}"
>
  <h3
    class="text-lg font-semibold mb-4 flex items-center gap-2 text-amber-400"
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
        d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
      />
    </svg>
    {t("flashcards.naming")}
  </h3>

  <div class="space-y-3">
    {#if needsDeckName}
      <div>
        <span class="block text-xs text-gray-400 mb-1">
          {t("flashcards.deckNameLabel")}
          <span class="text-red-400">*</span>
        </span>
        <input
          type="text"
          bind:value={generationStore.deckName}
          oninput={(event) => {
            generationStore.deckNameAuto =
              (event.currentTarget as HTMLInputElement).value.trim().length === 0;
          }}
          class="input-modern w-full text-sm"
          placeholder={t("flashcards.deckNamePlaceholder")}
        />
      </div>
    {:else}
      <div class="rounded-lg bg-violet-500/10 border border-violet-500/20 p-3">
        <div class="flex items-center gap-2 mb-1">
          <svg class="w-3.5 h-3.5 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span class="text-xs font-medium text-violet-300">{t("flashcards.deckNameAutoLabel")}</span>
        </div>
        <p class="text-[10px] text-gray-400">{t("flashcards.deckNameAutoDesc")}</p>
      </div>
    {/if}

  </div>
</div>

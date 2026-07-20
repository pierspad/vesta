<script lang="ts">
  import { locale } from "$lib/i18n";
  import type { CardFilterSettings } from "$lib/types/flashcardFilterTypes";

  interface Props {
    filters: CardFilterSettings;
    hasAnyFiles: boolean;
    hintLoadTargetFirst: string;
  }
  let { filters = $bindable(), hasAnyFiles, hintLoadTargetFirst }: Props = $props();

  let t = $derived($locale);
</script>

<div
  inert={!hasAnyFiles}
  title={!hasAnyFiles ? hintLoadTargetFirst : undefined}
  class="glass-card p-5 {!hasAnyFiles ? 'opacity-40' : ''}"
>
  <div class="flex items-center justify-between mb-3">
    <h3
      class="text-lg font-semibold flex items-center gap-2 text-amber-400"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2a1 1 0 01-.293.707L13 13.414V19a1 1 0 01-.553.894l-4 2A1 1 0 017 21v-7.586L3.293 6.707A1 1 0 013 6V4z" />
      </svg>
      {t("flashcards.cardFilters") || "Filtri Carte"}
    </h3>
    <button
      onclick={() => {
        filters.enabled = !filters.enabled;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative
        {filters.enabled ? 'bg-amber-500' : 'bg-gray-600'}"
      aria-label="Toggle card filters"
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
        {filters.enabled ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  <div class="space-y-3 transition-all duration-200 {!filters.enabled ? 'opacity-40 pointer-events-none' : ''}">
    <!-- Length Filter -->
    <div class="space-y-2">
      <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("flashcards.filterLength")}</span>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">{t("flashcards.filterMinChars")}</span>
            <button
              onclick={() => { filters.minCharsEnabled = !filters.minCharsEnabled; }}
              class="w-10 h-5 rounded-full transition-all duration-200 relative
                {filters.minCharsEnabled ? 'bg-amber-500' : 'bg-gray-600'}"
              aria-label="Enable min chars"
            >
              <div class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
                {filters.minCharsEnabled ? 'left-5' : 'left-0.5'}"></div>
            </button>
          </div>
          <div class="space-y-1.5">
            <div class="flex items-center gap-1">
              <input
                type="number" min="1"
                bind:value={filters.minChars}
                disabled={!filters.minCharsEnabled}
                class="input-modern w-full text-xs {!filters.minCharsEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
                placeholder="8"
              />
              <span class="text-xs text-gray-500 shrink-0">car.</span>
            </div>
            <input
              type="range" min="1" max="100" step="1"
              bind:value={filters.minChars}
              disabled={!filters.minCharsEnabled}
              class="slider-minimal w-full mt-1.5 transition-opacity duration-200 {!filters.minCharsEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
            />
          </div>
        </div>
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">{t("flashcards.filterMaxChars")}</span>
            <button
              onclick={() => { filters.maxCharsEnabled = !filters.maxCharsEnabled; }}
              class="w-10 h-5 rounded-full transition-all duration-200 relative
                {filters.maxCharsEnabled ? 'bg-amber-500' : 'bg-gray-600'}"
              aria-label="Enable max chars"
            >
              <div class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
                {filters.maxCharsEnabled ? 'left-5' : 'left-0.5'}"></div>
            </button>
          </div>
          <div class="space-y-1.5">
            <div class="flex items-center gap-1">
              <input
                type="number" min="1"
                bind:value={filters.maxChars}
                disabled={!filters.maxCharsEnabled}
                class="input-modern w-full text-xs {!filters.maxCharsEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
                placeholder="120"
              />
              <span class="text-xs text-gray-500 shrink-0">car.</span>
            </div>
            <input
              type="range" min="1" max="500" step="1"
              bind:value={filters.maxChars}
              disabled={!filters.maxCharsEnabled}
              class="slider-minimal w-full mt-1.5 transition-opacity duration-200 {!filters.maxCharsEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- Duration Filter -->
    <div class="space-y-2">
      <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("flashcards.filterDuration")}</span>
      <div class="grid grid-cols-2 gap-3">
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">{t("flashcards.filterMinDuration")}</span>
            <button
              onclick={() => { filters.minDurationEnabled = !filters.minDurationEnabled; }}
              class="w-10 h-5 rounded-full transition-all duration-200 relative
                {filters.minDurationEnabled ? 'bg-amber-500' : 'bg-gray-600'}"
              aria-label="Enable min duration"
            >
              <div class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
                {filters.minDurationEnabled ? 'left-5' : 'left-0.5'}"></div>
            </button>
          </div>
          <div class="space-y-1.5">
            <div class="flex items-center gap-1">
              <input
                type="number" min="0" step="100"
                bind:value={filters.minDurationMs}
                disabled={!filters.minDurationEnabled}
                class="input-modern w-full text-xs {!filters.minDurationEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
                placeholder="500"
              />
              <span class="text-xs text-gray-500 shrink-0">ms</span>
            </div>
            <input
              type="range" min="0" max="5000" step="100"
              bind:value={filters.minDurationMs}
              disabled={!filters.minDurationEnabled}
              class="slider-minimal w-full mt-1.5 transition-opacity duration-200 {!filters.minDurationEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
            />
          </div>
        </div>
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <span class="text-xs text-gray-400">{t("flashcards.filterMaxDuration")}</span>
            <button
              onclick={() => { filters.maxDurationEnabled = !filters.maxDurationEnabled; }}
              class="w-10 h-5 rounded-full transition-all duration-200 relative
                {filters.maxDurationEnabled ? 'bg-amber-500' : 'bg-gray-600'}"
              aria-label="Enable max duration"
            >
              <div class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
                {filters.maxDurationEnabled ? 'left-5' : 'left-0.5'}"></div>
            </button>
          </div>
          <div class="space-y-1.5">
            <div class="flex items-center gap-1">
              <input
                type="number" min="0" step="100"
                bind:value={filters.maxDurationMs}
                disabled={!filters.maxDurationEnabled}
                class="input-modern w-full text-xs {!filters.maxDurationEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
                placeholder="8000"
              />
              <span class="text-xs text-gray-500 shrink-0">ms</span>
            </div>
            <input
              type="range" min="0" max="30000" step="500"
              bind:value={filters.maxDurationMs}
              disabled={!filters.maxDurationEnabled}
              class="slider-minimal w-full mt-1.5 transition-opacity duration-200 {!filters.maxDurationEnabled ? 'opacity-40 cursor-not-allowed' : ''}"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- Sentence Combining -->
    <div class="mt-4 pt-4 border-t border-gray-800/50">
      <div class="flex items-center justify-between mb-3">
        <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("flashcards.combineSentences")}</span>
        <button
          onclick={() => (filters.combineSentences = !filters.combineSentences)}
          class="w-10 h-5 rounded-full transition-all duration-200 relative shrink-0 ml-3
            {filters.combineSentences ? 'bg-amber-500' : 'bg-gray-600'}"
          aria-label="Toggle sentence combining"
        >
          <div class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200
            {filters.combineSentences ? 'left-5' : 'left-0.5'}"></div>
        </button>
      </div>
      <div class="transition-opacity duration-200 {!filters.combineSentences ? 'opacity-40' : ''}">
        <span class="block text-xs text-gray-500 mb-1">{t("flashcards.continuationChars")}</span>
        <input
          type="text"
          bind:value={filters.continuationChars}
          disabled={!filters.combineSentences}
          class="input-modern w-full text-xs font-mono {!filters.combineSentences ? 'cursor-not-allowed' : ''}"
          placeholder=",、→"
        />
      </div>
    </div>
  </div>
</div>

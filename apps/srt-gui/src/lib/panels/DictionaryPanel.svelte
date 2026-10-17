<script lang="ts">
  import { locale, currentLanguage } from "$lib/i18n";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { dictionaryStore } from "$lib/stores/dictionaryStore.svelte";
  import {
    getDictionariesForLanguage,
    inferDictionaryForLanguages,
    getDictionarySearchTerms,
    formatDictionaryOptionLabel,
  } from "$lib/data/dictionaryProviders";
  import { formatDictionaryFlags, getLocalizedLanguageName } from "$lib/config/languageSearchIndex";
  import type { FlashcardDictionaryConfig } from "$lib/types/dictionaryTypes";

  interface Props {
    settings: FlashcardDictionaryConfig;
    studiedLanguage?: string;
    nativeLanguage?: string;
  }

  let {
    settings = $bindable(),
    studiedLanguage = "en",
    nativeLanguage = "it",
  }: Props = $props();

  let t = $derived($locale);
  let currentLocale = $derived($currentLanguage || "en");

  // Available options: user's studied language first, then all remaining languages sorted alphabetically
  let availableProviders = $derived(
    getDictionariesForLanguage(studiedLanguage, nativeLanguage, dictionaryStore.customDictionaries)
  );

  let providerOptions = $derived(
    availableProviders.map((p) => ({
      value: p.id,
      label: formatDictionaryOptionLabel(p, currentLocale),
      searchTerms: getDictionarySearchTerms(p),
    }))
  );

  // Auto-select best provider when studiedLanguage or nativeLanguage changes, if not explicitly set
  $effect(() => {
    if (studiedLanguage) {
      const inferred = inferDictionaryForLanguages(studiedLanguage, nativeLanguage);
      if (inferred) {
        const exists = availableProviders.some((p) => p.id === settings.providerId);
        if (!exists || !settings.providerId) {
          settings.providerId = inferred.id;
          settings.wordUrlTemplate = inferred.wordUrlTemplate;
          settings.sentenceUrlTemplate = inferred.sentenceUrlTemplate;
          settings.providerName = inferred.name;
        }
      }
    }
  });

  function onProviderSelect(newId: string) {
    settings.providerId = newId;
    const found =
      dictionaryStore.getDictionaryById(newId) ||
      availableProviders.find((p) => p.id === newId);
    if (found) {
      settings.wordUrlTemplate = found.wordUrlTemplate;
      settings.sentenceUrlTemplate = found.sentenceUrlTemplate;
      settings.providerName = found.name;
    }
  }

  let activeProvider = $derived(
    dictionaryStore.getDictionaryById(settings.providerId) ||
    availableProviders.find((p) => p.id === settings.providerId) ||
    availableProviders[0]
  );

  function handleCardClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target) return;
    if (
      target.closest(
        'button, input, select, textarea, a, [role="button"], [role="option"], [role="listbox"], .searchable-select-container, .searchable-select-dropdown'
      )
    ) {
      return;
    }
    settings.enabled = !settings.enabled;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  onclick={handleCardClick}
  class="glass-card p-5 cursor-pointer"
>
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-lg font-semibold flex items-center gap-2 text-indigo-400">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
      </svg>
      <span>{t("flashcards.dictionary.title")}</span>
    </h3>
    <button
      type="button"
      onclick={() => {
        settings.enabled = !settings.enabled;
      }}
      class="w-10 h-5 rounded-full transition-all duration-200 relative cursor-pointer {settings.enabled ? 'bg-indigo-500' : 'bg-gray-600'}"
      aria-label="Toggle dictionary"
    >
      <div
        class="absolute w-4 h-4 bg-white rounded-full top-0.5 transition-all duration-200 {settings.enabled ? 'left-5' : 'left-0.5'}"
      ></div>
    </button>
  </div>

  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    onclick={(e) => e.stopPropagation()}
    class="space-y-3 transition-all duration-200 {!settings.enabled ? 'opacity-40 pointer-events-none' : ''}"
  >
    <div>
      <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1">
        <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
        </svg>
        <span>{t("flashcards.dictionary.providerLabel")}</span>
      </span>
      <SearchableSelect
        options={providerOptions}
        value={settings.providerId}
        placeholder={t("flashcards.dictionary.placeholder")}
        onchange={onProviderSelect}
      />
    </div>

    {#if activeProvider}
      {@const flags = formatDictionaryFlags(activeProvider.targetLanguage, activeProvider.nativeLanguage)}
      {@const targetName = getLocalizedLanguageName(activeProvider.targetLanguage, currentLocale)}
      {@const nativeName = activeProvider.nativeLanguage ? getLocalizedLanguageName(activeProvider.nativeLanguage, currentLocale) : targetName}
      <div class="p-3 rounded-lg bg-white/5 border border-white/10 space-y-1.5">
        <div class="flex items-center justify-between gap-2">
          <span class="text-xs font-semibold text-gray-200 flex items-center gap-1.5">
            <span>{flags}</span>
            <span>{activeProvider.name}</span>
          </span>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">
            {targetName} {#if activeProvider.nativeLanguage && activeProvider.nativeLanguage !== activeProvider.targetLanguage}➔ {nativeName}{/if}
          </span>
        </div>
        {#if activeProvider.description}
          <p class="text-xs text-gray-400 leading-relaxed">
            {activeProvider.description}
          </p>
        {/if}
        <div class="flex flex-wrap gap-2 text-[11px] font-mono text-gray-400 pt-1">
          <span class="bg-black/30 px-2 py-0.5 rounded border border-white/5 truncate max-w-full" title={settings.wordUrlTemplate}>
            🔍 {t("flashcards.dictionary.wordLabel")}: {settings.wordUrlTemplate}
          </span>
          {#if settings.sentenceUrlTemplate}
            <span class="bg-black/30 px-2 py-0.5 rounded border border-white/5 truncate max-w-full" title={settings.sentenceUrlTemplate}>
              📖 {t("flashcards.dictionary.sentenceLabel")}: {settings.sentenceUrlTemplate}
            </span>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<script lang="ts">
  import { locale, currentLanguage } from "$lib/i18n";
  import { dictionaryStore } from "$lib/stores/dictionaryStore.svelte";
  import { builtInDictionaries, getDictionarySearchTerms } from "$lib/data/dictionaryProviders";
  import { formatDictionaryFlags, getLocalizedLanguageName } from "$lib/config/languageSearchIndex";
  import AddDictionaryModal from "$lib/modals/AddDictionaryModal.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import ToggleRow from "$lib/components/ToggleRow.svelte";

  let t = $derived($locale);
  let currentLocale = $derived($currentLanguage || "en");
  let showAddModal = $state(false);
  let searchQuery = $state("");

  let filteredBuiltIn = $derived.by(() => {
    const q = searchQuery.toLowerCase().trim();
    if (!q) return builtInDictionaries;
    return builtInDictionaries.filter((d) => getDictionarySearchTerms(d).includes(q));
  });

  function removeCustomDict(id: string) {
    dictionaryStore.removeCustomDictionary(id);
    snackbar.show(t("settings.dictionary.removed"), "info", 1500);
  }
</script>

<div class="glass-card p-6 space-y-6">
  <!-- Header with Toggle -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-white/10 pb-4">
    <div class="flex items-center gap-3">
      <div class="w-10 h-10 rounded-xl bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0 border border-indigo-500/30">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
        </svg>
      </div>
      <div>
        <h3 class="text-sm font-bold text-white">{t("settings.dictionary.title")}</h3>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={() => (showAddModal = true)}
        class="px-3 py-1.5 rounded-lg bg-indigo-600/80 hover:bg-indigo-500 text-white text-xs font-semibold flex items-center gap-1.5 shadow-md shadow-indigo-600/20 transition-all shrink-0"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        {t("settings.dictionary.addBtn")}
      </button>

      <ToggleRow
        label=""
        checked={dictionaryStore.enabled}
        onchange={() => dictionaryStore.toggleFeature()}
        accent="indigo"
      />
    </div>
  </div>

  {#if dictionaryStore.customDictionaries.length > 0}
    <!-- Custom Dictionaries Section -->
    <div class="space-y-3">
      <div class="flex items-center justify-between">
        <h4 class="text-xs font-bold uppercase tracking-wider text-indigo-400 flex items-center gap-2">
          <span>⭐ {t("settings.dictionary.customTitle")} ({dictionaryStore.customDictionaries.length})</span>
        </h4>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        {#each dictionaryStore.customDictionaries as custom (custom.id)}
          {@const flags = formatDictionaryFlags(custom.targetLanguage, custom.nativeLanguage)}
          {@const targetName = getLocalizedLanguageName(custom.targetLanguage, currentLocale)}
          {@const nativeName = custom.nativeLanguage ? getLocalizedLanguageName(custom.nativeLanguage, currentLocale) : targetName}
          <div class="p-3.5 rounded-lg bg-indigo-950/20 border border-indigo-500/30 flex flex-col justify-between gap-3 group">
            <div class="space-y-1">
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs font-bold text-white flex items-center gap-1.5">
                  <span>{flags}</span>
                  <span>{custom.name}</span>
                </span>
                <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">
                  {targetName}
                  {#if custom.nativeLanguage && custom.nativeLanguage !== custom.targetLanguage}
                    ➔ {nativeName}
                  {/if}
                </span>
              </div>
              {#if custom.description}
                <p class="text-[11px] text-gray-400 leading-relaxed">{custom.description}</p>
              {/if}
              <div class="text-[10px] font-mono text-gray-500 truncate pt-1">
                {custom.wordUrlTemplate}
              </div>
            </div>

            <div class="flex items-center justify-end border-t border-white/5 pt-2">
              <button
                type="button"
                onclick={() => removeCustomDict(custom.id)}
                class="text-[11px] text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 px-2 py-1 rounded transition-colors flex items-center gap-1"
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
                {t("settings.remove")}
              </button>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Built-in Dictionaries -->
  <div class="space-y-3">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <h4 class="text-xs font-bold uppercase tracking-wider text-gray-400">
        {t("settings.dictionary.standardTitle")} ({filteredBuiltIn.length})
      </h4>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder={t("settings.dictionary.searchPlaceholder")}
        class="bg-black/30 border border-white/10 rounded-lg px-3 py-1.5 text-xs text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500/50 w-full sm:w-64"
      />
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-2.5 max-h-[380px] overflow-y-auto pr-1">
      {#each filteredBuiltIn as dict (dict.id)}
        {@const flags = formatDictionaryFlags(dict.targetLanguage, dict.nativeLanguage)}
        {@const targetName = getLocalizedLanguageName(dict.targetLanguage, currentLocale)}
        {@const nativeName = dict.nativeLanguage ? getLocalizedLanguageName(dict.nativeLanguage, currentLocale) : targetName}
        <div class="p-3 rounded-lg bg-white/5 border border-white/10 flex flex-col justify-between gap-2 hover:border-white/20 transition-all">
          <div>
            <div class="flex items-center justify-between gap-2 mb-1">
              <span class="text-xs font-semibold text-gray-200 flex items-center gap-1.5">
                <span>{flags}</span>
                <span>{dict.name}</span>
              </span>
              <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-white/10 text-gray-300">
                {targetName}
                {#if dict.nativeLanguage && dict.nativeLanguage !== dict.targetLanguage}
                  ➔ {nativeName}
                {/if}
              </span>
            </div>
            {#if dict.description}
              <p class="text-[11px] text-gray-400 line-clamp-2 leading-relaxed">{dict.description}</p>
            {/if}
          </div>

          <div class="text-[10px] font-mono text-gray-500 truncate pt-1 border-t border-white/5">
            {dict.wordUrlTemplate}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<AddDictionaryModal open={showAddModal} onClose={() => (showAddModal = false)} />

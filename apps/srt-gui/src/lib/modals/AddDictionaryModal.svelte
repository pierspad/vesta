<script lang="ts">
  import { locale, currentLanguage } from "$lib/i18n";
  import { dictionaryStore } from "$lib/stores/dictionaryStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { getLanguageOptions } from "$lib/config/languages";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();
  let t = $derived($locale);
  let currentLocale = $derived($currentLanguage || "en");

  let name = $state("");
  let targetLanguage = $state("en");
  let nativeLanguage = $state("it");
  let wordUrlTemplate = $state("");
  let sentenceUrlTemplate = $state("");
  let description = $state("");

  let targetLangOptions = $derived([
    {
      value: "*",
      label: t("settings.dictionary.modal.allLanguages"),
      searchTerms: "all universal tutte le lingue global",
    },
    ...getLanguageOptions(currentLocale),
  ]);

  let nativeLangOptions = $derived([
    {
      value: "",
      label: t("settings.dictionary.modal.monolingual"),
      searchTerms: "none monolingual nessuna monolingua",
    },
    ...getLanguageOptions(currentLocale),
  ]);

  let canSave = $derived(
    name.trim().length > 0 &&
    wordUrlTemplate.trim().length > 0 &&
    wordUrlTemplate.includes("{word}")
  );

  function resetForm() {
    name = "";
    targetLanguage = "en";
    nativeLanguage = "it";
    wordUrlTemplate = "";
    sentenceUrlTemplate = "";
    description = "";
  }

  function handleSave() {
    if (!canSave) return;
    const res = dictionaryStore.addCustomDictionary({
      name,
      targetLanguage,
      nativeLanguage: nativeLanguage.trim() || undefined,
      wordUrlTemplate,
      sentenceUrlTemplate: sentenceUrlTemplate.trim() || undefined,
      description: description.trim() || undefined,
    });

    if (res.success) {
      snackbar.show(t("settings.dictionary.modal.addedSuccess"), "success", 2000);
      resetForm();
      onClose();
    } else {
      snackbar.show(res.error || t("settings.dictionary.modal.saveError"), "error", 3000);
    }
  }

  function handleCancel() {
    resetForm();
    onClose();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    role="dialog"
    tabindex="-1"
    onmousedown={(e) => {
      if (e.target === e.currentTarget) handleCancel();
    }}
  >
    <div
      class="w-full max-w-xl max-h-[92vh] overflow-hidden animate-fade-in shadow-2xl border border-white/20 bg-gray-900/98 backdrop-blur-xl rounded-xl flex flex-col"
      role="presentation"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="p-6 border-b border-white/5 bg-white/5 flex items-center justify-between">
        <h3 class="text-lg font-bold text-white flex items-center gap-2">
          <svg class="w-5 h-5 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
          </svg>
          {t("settings.dictionary.modal.title")}
        </h3>
        <button
          type="button"
          onclick={handleCancel}
          aria-label={t("common.close")}
          class="text-gray-400 hover:text-white p-1 rounded-lg hover:bg-white/10 transition-colors"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="p-6 flex-1 overflow-y-auto space-y-4">
        <div>
          <label for="dict-name" class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
            {t("settings.dictionary.modal.nameLabel")} <span class="text-rose-400">*</span>
          </label>
          <input
            id="dict-name"
            type="text"
            bind:value={name}
            placeholder={t("settings.dictionary.modal.namePlaceholder")}
            class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
          />
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <span class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
              {t("settings.dictionary.modal.targetLang")} <span class="text-rose-400">*</span>
            </span>
            <SearchableSelect
              options={targetLangOptions}
              value={targetLanguage}
              onchange={(v) => (targetLanguage = v)}
              placeholder={t("settings.dictionary.modal.targetLangPlaceholder")}
            />
          </div>

          <div>
            <span class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
              {t("settings.dictionary.modal.nativeLang")}
            </span>
            <SearchableSelect
              options={nativeLangOptions}
              value={nativeLanguage}
              onchange={(v) => (nativeLanguage = v)}
              placeholder={t("settings.dictionary.modal.nativeLangPlaceholder")}
            />
          </div>
        </div>

        <div>
          <label for="dict-word-url" class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
            {t("settings.dictionary.modal.wordUrl")} <span class="text-rose-400">*</span>
          </label>
          <input
            id="dict-word-url"
            type="text"
            bind:value={wordUrlTemplate}
            placeholder={t("settings.dictionary.modal.wordUrlPlaceholder")}
            class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white font-mono text-xs focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
          />
          <p class="text-[11px] text-gray-400 mt-1">
            {t("settings.dictionary.modal.wordUrlHint")}
          </p>
        </div>

        <div>
          <label for="dict-sentence-url" class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
            {t("settings.dictionary.modal.sentenceUrl")}
          </label>
          <input
            id="dict-sentence-url"
            type="text"
            bind:value={sentenceUrlTemplate}
            placeholder={t("settings.dictionary.modal.sentenceUrlPlaceholder")}
            class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white font-mono text-xs focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600"
          />
          <p class="text-[11px] text-gray-400 mt-1">
            {t("settings.dictionary.modal.sentenceUrlHint")}
          </p>
        </div>

        <div>
          <label for="dict-desc" class="block text-xs font-bold text-gray-400 uppercase tracking-wide mb-1.5">
            {t("settings.dictionary.modal.desc")}
          </label>
          <textarea
            id="dict-desc"
            bind:value={description}
            rows={2}
            placeholder={t("settings.dictionary.modal.descPlaceholder")}
            class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all placeholder-gray-600 resize-none"
          ></textarea>
        </div>
      </div>

      <div class="p-4 border-t border-white/5 bg-white/5 flex items-center justify-end gap-3">
        <button
          type="button"
          onclick={handleCancel}
          class="px-4 py-2 rounded-lg text-sm text-gray-300 hover:text-white hover:bg-white/5 transition-all font-semibold"
        >
          {t("common.cancel")}
        </button>
        <button
          type="button"
          onclick={handleSave}
          disabled={!canSave}
          class="px-5 py-2 rounded-lg text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 disabled:cursor-not-allowed text-white font-semibold shadow-lg shadow-indigo-600/20 transition-all flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
          {t("settings.dictionary.modal.saveBtn")}
        </button>
      </div>
    </div>
  </div>
{/if}

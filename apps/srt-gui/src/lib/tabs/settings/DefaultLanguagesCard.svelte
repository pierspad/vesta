<script lang="ts">
  import { locale } from "$lib/i18n";
  import { languages } from "$lib/config/languages";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";

  interface Props {
    defaultNativeLanguage: string;
    defaultFlashcardsLanguage: string;
    defaultTargetLanguage: string;
    defaultTranscribeLanguage: string;
    onChangeNative: (code: string) => void;
    onChangeFlashcards: (code: string) => void;
    onChangeTarget: (code: string) => void;
    onChangeTranscribe: (code: string) => void;
  }

  let {
    defaultNativeLanguage,
    defaultFlashcardsLanguage,
    defaultTargetLanguage,
    defaultTranscribeLanguage,
    onChangeNative,
    onChangeFlashcards,
    onChangeTarget,
    onChangeTranscribe,
  }: Props = $props();

  let t = $derived($locale);
</script>

<!-- Keep the card lifted so language dropdowns paint over the cards below. -->
<div class="glass-card p-6 relative z-30">
  <div class="grid grid-cols-1 md:grid-cols-2 2xl:grid-cols-4 gap-5">
    <div class="relative z-40 rounded-xl border border-transparent bg-white/5 p-5">
      <div class="flex items-center justify-between gap-3 mb-4">
        <span class="flex items-center gap-2 text-sm font-semibold text-white">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-300">
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a4 4 0 110 8 4 4 0 010-8zM5 21a7 7 0 0114 0"/></svg>
          </span>
          {t("settings.copy.nativeLanguage")}
        </span>
        <span class="text-3xl">{languages.find((lang) => lang.code === defaultNativeLanguage)?.flag || "🌐"}</span>
      </div>
      <LanguageSelect
        className="language-select"
        noResultsText={t("common.noResults")}
        value={defaultNativeLanguage}
        onchange={onChangeNative}
        placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
      />
      <p class="mt-3 text-xs leading-relaxed text-gray-500">{t("settings.copy.nativeLanguageHint")}</p>
    </div>

    <div class="relative z-30 rounded-xl border border-transparent bg-white/5 p-5">
      <div class="flex items-center justify-between gap-3 mb-4">
        <span class="flex items-center gap-2 text-sm font-semibold text-white">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-500/15 text-violet-300">文</span>
          {t("settings.copy.studyingLanguage")}
        </span>
        <span class="text-3xl">{languages.find((lang) => lang.code === defaultFlashcardsLanguage)?.flag || "🌐"}</span>
      </div>
      <LanguageSelect
        className="language-select"
        noResultsText={t("common.noResults")}
        value={defaultFlashcardsLanguage}
        onchange={onChangeFlashcards}
        placeholder={t("flashcards.noteTypeLanguagePlaceholder")}
      />
      <p class="mt-3 text-xs leading-relaxed text-gray-500">{t("settings.copy.studyingLanguageHint")}</p>
    </div>

    <div class="relative z-20 rounded-xl border border-transparent bg-white/5 p-5">
      <div class="flex items-center justify-between gap-3 mb-4">
        <span class="flex items-center gap-2 text-sm font-semibold text-white">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300">
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 7h11m0 0-3-3m3 3-3 3M17 17H6m0 0 3 3m-3-3 3-3"/></svg>
          </span>
          {t("settings.copy.translationLanguage")}
        </span>
        <span class="text-3xl">{languages.find((lang) => lang.code === defaultTargetLanguage)?.flag || "🌐"}</span>
      </div>
      <LanguageSelect
        className="language-select"
        noResultsText={t("common.noResults")}
        value={defaultTargetLanguage}
        onchange={onChangeTarget}
        placeholder={t("translate.targetLang")}
      />
      <p class="mt-3 text-xs leading-relaxed text-gray-500">{t("settings.copy.translationLanguageHint")}</p>
    </div>

    <div class="relative z-10 rounded-xl border border-transparent bg-white/5 p-5">
      <div class="flex items-center justify-between gap-3 mb-4">
        <span class="flex items-center gap-2 text-sm font-semibold text-white">
          <span class="flex h-8 w-8 items-center justify-center rounded-lg bg-amber-500/15 text-amber-300">
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3a3 3 0 00-3 3v6a3 3 0 006 0V6a3 3 0 00-3-3zM5 11a7 7 0 0014 0M12 18v3m-4 0h8"/></svg>
          </span>
          {t("settings.copy.transcription")}
        </span>
        <span class="text-3xl">{defaultTranscribeLanguage === "auto" ? "🌐" : languages.find((lang) => lang.code === defaultTranscribeLanguage)?.flag || "🌐"}</span>
      </div>
      <LanguageSelect
        className="language-select"
        noResultsText={t("common.noResults")}
        autoDetect
        value={defaultTranscribeLanguage}
        onchange={onChangeTranscribe}
        placeholder={t("transcribe.sourceLanguage")}
      />
      <p class="mt-3 text-xs leading-relaxed text-gray-500">{t("settings.copy.transcriptionLanguageHint")}</p>
    </div>
  </div>
</div>

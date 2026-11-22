<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import LanguageSelect from "$lib/components/LanguageSelect.svelte";
  import type { SrtFileInfo } from "$lib/services/translate";

  interface Props {
    targetLang: string;
    batchSize: number;
    resumeOverlap: number;
    titleContext: string;
    contextDraft: string;
    contextDraftPath: string;
    contextLoading: boolean;
    contextError: string;
    inputPath: string;
    fileInfo: SrtFileInfo | null;
    useTiers: boolean;
    isTranslating: boolean;
    highlightClass: string;
    onSuggestContext: () => void;
    onApplyContext: () => void;
  }

  let {
    targetLang = $bindable(),
    batchSize = $bindable(),
    resumeOverlap = $bindable(),
    titleContext = $bindable(),
    contextDraft = $bindable(),
    contextDraftPath,
    contextLoading,
    contextError,
    inputPath,
    fileInfo,
    useTiers,
    isTranslating,
    highlightClass,
    onSuggestContext,
    onApplyContext,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card relative z-10 p-5 shrink-0 {highlightClass}">
  <h3 class="text-lg font-semibold mb-4 flex items-center gap-2 text-cyan-400">
    <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" />
    </svg>
    {t("translate.options")}
  </h3>
  <fieldset disabled={isTranslating || contextLoading} class="min-w-0 space-y-4">
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
      <div class={uiMode.expertMode ? "lg:col-span-4" : "lg:col-span-12"}>
        <label for="target-lang" class="flex items-center gap-1.5 text-sm text-gray-400 mb-1 font-medium">
          <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
          </svg>
          <span>{t("translate.targetLang")}</span>
        </label>
        <LanguageSelect
          noResultsText={t("common.noResults")}
          value={targetLang}
          onchange={(v) => (targetLang = v)}
          placeholder={t("translate.targetLang")}
        />
      </div>

      {#if uiMode.expertMode}
        <div class="lg:col-span-4">
          <div class="flex items-center justify-between mb-1">
            <span class="flex items-center gap-1.5 text-sm text-gray-400 font-medium">
              <svg aria-hidden="true" class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h10" />
              </svg>
              <span>{t("translate.batchSizeExpert")}</span>
            </span>
            <span class="text-gray-200 font-mono bg-white/10 px-2 py-0.5 rounded text-xs shrink-0">
              {batchSize} {t("translate.subPerBatch")}
            </span>
          </div>
          <input type="range" min="1" max="100" bind:value={batchSize} class="slider-resource w-full cursor-pointer" />
          <div class="relative mt-1" style="height: 18px;">
            {#each [1, 20, 40, 60, 80, 100] as v}
              {@const pct = ((v - 1) / 99) * 100}
              <div class="absolute flex flex-col items-center gap-0.5" style="left: {pct}%; transform: translateX(-50%);">
                <div class="w-px h-1 {batchSize === v ? 'bg-white/60' : 'bg-white/20'}"></div>
                <span class="text-[9px] {batchSize === v ? 'text-white/70' : 'text-white/25'}">{v}</span>
              </div>
            {/each}
          </div>
        </div>

        <div class="lg:col-span-4">
          <div class="flex items-center justify-between mb-1">
            <span class="flex items-center gap-1.5 text-sm text-gray-400 font-medium">
              <svg aria-hidden="true" class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
              </svg>
              <span>{t("translate.resumeOverlapExpert")}</span>
            </span>
            <span class="text-gray-200 font-mono bg-white/10 px-2 py-0.5 rounded text-xs shrink-0">
              {resumeOverlap} sub
            </span>
          </div>
          <input type="range" min="0" max="15" bind:value={resumeOverlap} class="slider-resource w-full cursor-pointer" />
          <div class="relative mt-1" style="height: 18px;">
            {#each [0, 3, 6, 9, 12, 15] as v}
              {@const pct = (v / 15) * 100}
              <div class="absolute flex flex-col items-center gap-0.5" style="left: {pct}%; transform: translateX(-50%);">
                <div class="w-px h-1 {resumeOverlap === v ? 'bg-white/60' : 'bg-white/20'}"></div>
                <span class="text-[9px] {resumeOverlap === v ? 'text-white/70' : 'text-white/25'}">{v}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <details open={uiMode.expertMode} class="pt-3 border-t border-white/5">
      <summary id="translation-context-label" class="cursor-pointer text-sm text-gray-400 font-medium">
        <span class="inline-flex items-center gap-1.5 align-middle">
          <svg aria-hidden="true" class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8h10M7 12h6m-6 4h8M5 4h14a2 2 0 012 2v12a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2z" />
          </svg>
          <span>{t("translate.context")}</span>
          <span class="font-normal text-gray-500">({t("translate.contextOptional")})</span>
        </span>
      </summary>
      <div class="mt-3">
        <div class="flex flex-wrap items-center justify-between gap-2 mb-2">
          <button
            type="button"
            onclick={onSuggestContext}
            disabled={!fileInfo || !useTiers || contextLoading || isTranslating || aiStore.killSwitchActive}
            class="btn-secondary px-3 py-1.5 text-xs disabled:opacity-40"
          >
            {t(contextLoading ? "translate.contextGenerating" : "translate.suggestContext")}
          </button>
        </div>
        {#if fileInfo}
          <p class="text-[11px] text-gray-400 mb-2">
            {getFileName(fileInfo.path)} · {t("translate.subtitleCoverage")}: {Math.floor(fileInfo.coverage_end_ms / 60000)}:{String(Math.floor(fileInfo.coverage_end_ms / 1000) % 60).padStart(2, "0")}
          </p>
        {/if}
        <textarea
          id="context-input"
          aria-labelledby="translation-context-label"
          bind:value={titleContext}
          rows="2"
          placeholder={t("translate.contextPlaceholder")}
          class="input-modern w-full text-xs min-h-[4rem] resize-y"
        ></textarea>
        {#if contextError}<p class="mt-2 text-xs text-red-300" role="alert">{contextError}</p>{/if}
        {#if contextDraft && contextDraftPath === inputPath}
          <div class="mt-3 rounded-xl border border-cyan-500/25 bg-cyan-500/5 p-3">
            <label for="context-draft" class="block text-xs text-cyan-300 mb-2">{t("translate.contextDraft")}</label>
            <textarea id="context-draft" bind:value={contextDraft} rows="4" class="input-modern w-full text-xs resize-y"></textarea>
            <button type="button" class="btn-secondary mt-2 px-3 py-1.5 text-xs" onclick={onApplyContext}>{t("translate.applyContext")}</button>
          </div>
        {/if}
      </div>
    </details>
  </fieldset>
</div>

<script lang="ts">
  import { locale } from "$lib/i18n";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import ProgressFill from "$lib/components/ProgressFill.svelte";
  import ApiKeysCard from "$lib/panels/ApiKeysCard.svelte";
  import TranslationTiers from "$lib/components/TranslationTiers.svelte";
  import { ocrModelsStore } from "$lib/stores/ocrModelsStore.svelte";
  import { ITT_TIERS_UPDATED_EVENT, loadIttTiers, saveIttTiers } from "$lib/config/ittTiers";
  import type { ApiKeyConfig } from "$lib/config/apiKeys";

  interface Props {
    active: boolean;
    s: (key: any) => string;
    apiKeys: ApiKeyConfig[];
    highlightedModelId: string | null;
    onAddKey: (defaultProvider?: string) => void;
    onEditKey: (id: string) => void;
    onDeleteKey: (id: string) => void;
    onSetDefault: (id: string) => void;
  }

  let {
    active,
    s,
    apiKeys,
    highlightedModelId,
    onAddKey,
    onEditKey,
    onDeleteKey,
    onSetDefault,
  }: Props = $props();

  let t = $derived($locale);
  let ocrStore = ocrModelsStore;
  let ittTiersRef = $state<{ triggerAddTier: () => void } | null>(null);
</script>

{#if !aiStore.killSwitchActive}
  <div id="itt-endpoints">
    <ApiKeysCard
      title={s("apiKeysSaved")}
      addButtonLabel={s("addProviderButton")}
      defaultProvider="custom"
      {apiKeys}
      {onAddKey}
      {onEditKey}
      {onDeleteKey}
      {onSetDefault}
    />
  </div>

  <!-- ITT / Vision Priority Chain -->
  <div id="itt-tiers" class="glass-card flex flex-col mb-6 mt-4 relative z-20 {highlightedModelId === 'itt-tiers' ? 'requirement-pulse' : ''}">
    <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-lg bg-violet-500/20 text-violet-300 flex items-center justify-center shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h10M4 18h6" />
          </svg>
        </div>
        <div>
          <h3 class="text-sm font-bold text-white">{t("settings.ittTiersTitle")}</h3>
          <p class="text-xs text-gray-400 mt-0.5">{t("settings.ittTiersDesc")}</p>
        </div>
      </div>
      <button
        type="button"
        onclick={() => ittTiersRef?.triggerAddTier()}
        class="inline-flex items-center justify-center gap-2 rounded-lg bg-violet-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-violet-500/20 hover:bg-violet-400 transition-colors cursor-pointer shrink-0"
      >
        + {t("tiers.addTier")}
      </button>
    </div>
    <div class="p-4">
      <TranslationTiers
        bind:this={ittTiersRef}
        {active}
        capability="itt"
        loadTiersFn={loadIttTiers}
        saveTiersFn={saveIttTiers}
        updatedEvent={ITT_TIERS_UPDATED_EVENT}
        autoCreateDefault={false}
      />
    </div>
  </div>
{/if}

<!-- Offline OCR Models (PaddleOCR) -->
<div id="ocr-models" class="glass-card p-5 {aiStore.killSwitchActive ? '' : 'mt-6'} {highlightedModelId === 'ocr-models' ? 'requirement-pulse' : ''}" role="group" aria-label={t("settings.ocrModelsTitle")}>
  <div class="flex items-center gap-3 mb-4">
    <div class="w-8 h-8 rounded-lg bg-violet-500/20 flex items-center justify-center text-violet-200">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5a2 2 0 012-2h14a2 2 0 012 2v14a2 2 0 01-2 2H5a2 2 0 01-2-2V5zm4 10h10M7 11h6m-4-4h.01" />
      </svg>
    </div>
    <div class="flex-1">
      <h3 class="text-sm font-bold text-white">{t("settings.ocrModelsTitle")}</h3>
      <p class="text-xs text-gray-400 mt-0.5">{t("settings.ocrModelsDesc")}</p>
    </div>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
    <div class="p-4 rounded-xl bg-cyan-500/10 border border-cyan-500/25">
      <p class="text-xs uppercase tracking-wide text-cyan-300/70 mb-1">{t("settings.modelsDownloadedLocally")}</p>
      <p class="text-2xl font-bold text-white">{ocrStore.downloadedOcrCount}/{ocrStore.ocrModels.length}</p>
    </div>
    <div class="p-4 rounded-xl bg-white/5 border border-white/10">
      <p class="text-xs uppercase tracking-wide text-gray-500 mb-1">{t("settings.default")}</p>
      <p class="text-sm font-bold text-white truncate">{ocrStore.ocrModels.find((m) => m.id === ocrStore.defaultOcrModel)?.name ?? ocrStore.defaultOcrModel}</p>
    </div>
    <div class="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/25">
      <p class="text-xs uppercase tracking-wide text-emerald-300/70 mb-1">{t("settings.ready")}</p>
      <p class="text-2xl font-bold text-white">{ocrStore.downloadedOcrCount > 0 ? t("common.yes") : t("common.no")}</p>
    </div>
  </div>

  <div class="grid grid-cols-1 sm:grid-cols-3 gap-3" role="radiogroup" aria-label={t("settings.ocrModelsTitle")}>
    {#each ocrStore.ocrModels as model (model.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        onclick={() => ocrStore.handleModelClick(model)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            ocrStore.handleModelClick(model);
          }
        }}
        role="radio"
        aria-checked={ocrStore.defaultOcrModel === model.id}
        tabindex="0"
        class="group relative overflow-hidden min-h-[8.5rem] p-4 rounded-xl text-center transition-colors duration-200 border cursor-pointer select-none
          {ocrStore.defaultOcrModel === model.id && model.downloaded
          ? 'bg-cyan-500/20 border-cyan-500/50 text-white shadow-[0_0_15px_rgba(6,182,212,0.15)]'
          : model.downloaded
            ? 'bg-white/10 hover:bg-white/20 border-white/20 text-gray-200'
            : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-500 opacity-60 hover:opacity-90'}"
        title={model.downloaded ? t("settings.whisperDownloadedHint") : t("settings.whisperNotDownloadedHint")}
      >
        {#if ocrStore.isDownloading && ocrStore.downloadingModelId === model.id}
          <ProgressFill value={ocrStore.progress} />
        {/if}
        <div class="absolute top-2 right-2 z-10 flex items-center">
          {#if !model.downloaded}
            {#if ocrStore.downloadingModelId === model.id}
              <button
                type="button"
                onclick={(e) => { e.stopPropagation(); void ocrStore.cancelModelDownload(); }}
                disabled={ocrStore.isCancellingDownload}
                class="text-red-400 hover:text-red-300 transition-colors p-1 bg-red-500/15 hover:bg-red-500/25 rounded-md border border-red-500/30 flex items-center justify-center cursor-pointer shadow-sm"
                title={t("settings.stopModelDownload")}
                aria-label={t("settings.stopModelDownload")}
              >
                {#if ocrStore.isCancellingDownload}
                  <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                  </svg>
                {:else}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 6h12v12H6z" />
                  </svg>
                {/if}
              </button>
            {:else}
              <button
                type="button"
                onclick={(e) => { e.stopPropagation(); void ocrStore.downloadModel(model.id, true); }}
                class="text-amber-400 hover:text-cyan-300 transition-colors p-1 bg-amber-500/10 hover:bg-cyan-500/20 border border-amber-500/20 hover:border-cyan-500/40 rounded-md cursor-pointer shadow-sm"
                title={t("settings.download")}
                aria-label={t("settings.download")}
                disabled={ocrStore.busy}
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                </svg>
              </button>
            {/if}
          {:else}
            <button
              type="button"
              onclick={(e) => { e.stopPropagation(); void ocrStore.uninstallModel(model.id); }}
              disabled={ocrStore.busy}
              class="opacity-0 group-hover:opacity-100 focus:opacity-100 text-gray-400 hover:text-red-400 hover:bg-red-500/15 p-1 rounded-md transition-colors cursor-pointer border border-transparent hover:border-red-500/30"
              title={t("settings.remove")}
              aria-label={t("settings.remove")}
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
            </button>
          {/if}
        </div>

        <div class="mx-auto mb-2 flex h-11 w-11 items-center justify-center rounded-xl border border-white/10 bg-violet-500/15 text-violet-200">
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h10M4 18h6" />
          </svg>
        </div>
        <div class="font-bold text-sm leading-snug">{model.name}</div>
        <div class="text-[10px] text-gray-500 mt-1">{model.size}</div>
        {#if !model.downloaded}
          <div class="text-[9px] text-amber-400/70 mt-0.5">
            {#if ocrStore.downloadingModelId === model.id}
              {t("settings.downloading")} {ocrStore.progress > 0 ? `${ocrStore.progress}%` : ""}
            {:else}
              {t("settings.notDownloaded")}
            {/if}
          </div>
        {:else if ocrStore.defaultOcrModel === model.id}
          <div class="text-[9px] text-cyan-400 mt-0.5 font-bold">{t("settings.default")}</div>
        {:else}
          <div class="text-[9px] text-emerald-400/80 mt-0.5">{t("settings.ready")}</div>
        {/if}
      </div>
    {/each}
  </div>
</div>

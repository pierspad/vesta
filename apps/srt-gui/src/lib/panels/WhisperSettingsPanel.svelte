<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { locale } from "$lib/i18n";
  import ApiKeysCard from "$lib/panels/ApiKeysCard.svelte";
  import TranscribeTiers from "$lib/components/TranscribeTiers.svelte";
  import WhisperModelSelector from "$lib/components/WhisperModelSelector.svelte";
  import { DEFAULT_VAD_MODEL_ID } from "$lib/config/vadSelection";
  import type { ApiKeyConfig } from "$lib/config/apiKeys";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";

  interface Props {
    s: (key: any) => string;
    apiKeys: ApiKeyConfig[];
    onAddKey: (defaultProvider: string) => void;
    onEditKey: (id: string) => void;
    onDeleteKey: (id: string) => void;
    onSetDefault: (id: string) => void;
    /** Shared with the llm section's refinement-prompt editor -- genuinely
     * parent-owned, see whisperModelsStore.svelte.ts for why it's a prop
     * here instead of living in the store. */
    highlightedModelId: string | null;
    /** "local" | "cloud" -- kept in SettingsTab.svelte (there's currently no
     * UI anywhere that sets it to "cloud", but that's a pre-existing
     * condition, not something to silently change in a structural refactor). */
    whisperEngine: string;
  }

  let { s, apiKeys, onAddKey, onEditKey, onDeleteKey, onSetDefault, highlightedModelId, whisperEngine }: Props = $props();

  let t = $derived($locale);
  let store = whisperModelsStore;
  let transcribeTiersRef = $state<any>(null);

  function handleModelDblClick(model: { id: string; downloaded: boolean }) {
    if (!model.downloaded && !store.isDownloading) {
      void store.downloadModel(model.id, true);
    } else if (model.downloaded) {
      store.setDefaultWhisperModel(model.id);
    }
  }

  // The only dispatchers of these two events are TranscribeTiers.svelte
  // (rendered as our own child below) and this panel's own model grid, so
  // it's safe for this listener pair to live and die with this panel --
  // unlike the Tauri "transcribe-progress" listener (a real backend
  // subscription that must survive section navigation), which stays
  // registered in SettingsTab.svelte's app-lifetime onMount instead. See
  // [[vesta-settings-refactor]].
  function handleDownloadWhisperModelEvent(e: Event) {
    const customEvent = e as CustomEvent<{ modelId: string }>;
    const modelId = customEvent.detail?.modelId;
    if (modelId) void store.downloadModel(modelId, true);
  }

  function handleUninstallWhisperModelEvent(e: Event) {
    const customEvent = e as CustomEvent<{ modelId: string }>;
    const modelId = customEvent.detail?.modelId;
    if (modelId) void store.uninstallModel(modelId);
  }

  onMount(() => {
    window.addEventListener("vesta-download-whisper-model", handleDownloadWhisperModelEvent);
    window.addEventListener("vesta-uninstall-whisper-model", handleUninstallWhisperModelEvent);
  });

  onDestroy(() => {
    window.removeEventListener("vesta-download-whisper-model", handleDownloadWhisperModelEvent);
    window.removeEventListener("vesta-uninstall-whisper-model", handleUninstallWhisperModelEvent);
  });
</script>

<ApiKeysCard
  title={s("apiKeysSaved")}
  addButtonLabel={s("addProviderButton")}
  defaultProvider="groq"
  {apiKeys}
  {onAddKey}
  {onEditKey}
  {onDeleteKey}
  {onSetDefault}
/>

<!-- Whisper Tiers (priority list + failover) -->
<div class="glass-card flex flex-col mb-6 mt-10">
  <div class="p-4 border-b border-white/5 flex items-center justify-between gap-3 w-full">
    <div class="flex items-center gap-3">
      <div class="w-9 h-9 rounded-lg bg-indigo-500/20 text-indigo-300 flex items-center justify-center shrink-0">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h10M4 18h6" />
        </svg>
      </div>
      <div>
        <h3 class="text-sm font-bold text-white">{t("transcribe.tiers.cardTitle")}</h3>
      </div>
    </div>
    <button
      type="button"
      onclick={() => transcribeTiersRef?.triggerAddTier()}
      class="inline-flex items-center justify-center gap-2 rounded-lg bg-indigo-500 px-3.5 py-2 text-xs font-bold text-white shadow-lg shadow-indigo-500/20 hover:bg-indigo-400 transition-colors cursor-pointer"
    >
      + {t("tiers.addTier")}
    </button>
  </div>
  <div class="p-4">
    <TranscribeTiers bind:this={transcribeTiersRef} />
  </div>
</div>



{#if whisperEngine === "local"}
<div class="mt-6 glass-card p-5" role="group" oncontextmenu={(e) => store.openWhisperPanelContextMenu(e)}>
  <div class="flex items-center gap-3 mb-4">
    <div class="w-8 h-8 rounded-lg bg-cyan-500/20 flex items-center justify-center text-cyan-200">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    </div>
    <div class="flex-1">
      <h3 class="text-sm font-bold text-white">{t("transcribe.whisperModel")}</h3>
    </div>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
    <div class="p-4 rounded-xl bg-cyan-500/10 border border-cyan-500/25">
      <p class="text-xs uppercase tracking-wide text-cyan-300/70 mb-1">{t("settings.modelsDownloadedLocally")}</p>
      <p class="text-2xl font-bold text-white">{store.downloadedWhisperCount}/{store.whisperModels.length}</p>
    </div>
    <div class="p-4 rounded-xl bg-white/5 border border-white/10">
      <p class="text-xs uppercase tracking-wide text-gray-500 mb-1">{t("settings.default")}</p>
      <p class="text-2xl font-bold text-white">{store.defaultWhisperModel ? (t(`transcribe.model${store.defaultWhisperModel.charAt(0).toUpperCase()}${store.defaultWhisperModel.slice(1)}`) || store.defaultWhisperModel) : ""}</p>
    </div>
    <div class="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/25">
      <p class="text-xs uppercase tracking-wide text-emerald-300/70 mb-1">{t("settings.ready")}</p>
      <p class="text-2xl font-bold text-white">{store.downloadedWhisperCount > 0 ? t("common.yes") : t("common.no")}</p>
    </div>
  </div>

  {#if store.isDownloading && store.downloadingModelId}
    <div class="mb-3 text-xs text-gray-400">
      {t("settings.modelDownloading", { model: store.downloadingModelId }) || `Downloading model: ${store.downloadingModelId}`}
      {#if store.progress > 0}
        <span class="text-cyan-300 ml-1">{store.progress}%</span>
      {/if}
    </div>
  {/if}

  <WhisperModelSelector
    models={store.whisperModels}
    value={store.defaultWhisperModel}
    disabled={store.isDownloading}
    onselect={(model) => store.handleWhisperModelClick(model)}
    oncontextmenu={(event, model) => store.openContextMenu(event, model)}
    ondblclick={handleModelDblClick}
  />

  <!-- Silero VAD add-ons: two downloadable variants + an optional custom model.
       Clicking a downloaded variant makes it the active one (same pattern as
       the Whisper model grid above); the active choice is what TranscribeTab
       resolves into transcribe_start's config. -->
  <div class="mt-4 space-y-2">
    {#each store.vadModels as model (model.id)}
      {@const isActive = !store.vadSelection.customPath && store.vadSelection.modelId === model.id}
      <div class="p-4 rounded-xl border flex items-center justify-between gap-4 {isActive
        ? 'bg-emerald-500/10 border-emerald-500/25'
        : 'bg-white/5 border-white/10'}">
        <button
          type="button"
          onclick={() => store.handleVadModelClick(model)}
          disabled={store.downloadingVadId !== null}
          class="min-w-0 text-left flex-1 cursor-pointer disabled:cursor-default"
        >
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-sm font-bold text-white">Silero VAD {model.id}</span>
            {#if model.id === DEFAULT_VAD_MODEL_ID}
              <span class="text-[9px] text-gray-500 uppercase tracking-wide">{t("settings.default")}</span>
            {/if}
            <span class="text-[10px] text-gray-500">{model.size}</span>
            {#if model.downloaded}
              {#if isActive}
                <span class="text-[9px] font-bold text-emerald-400 uppercase tracking-wide">{t("settings.ready")}</span>
              {/if}
            {:else if store.downloadingVadId === model.id}
              <span class="text-[9px] text-amber-400/70">{t("settings.downloading")} {store.progress > 0 ? `${store.progress}%` : ""}</span>
            {:else}
              <span class="text-[9px] text-amber-400/70">{t("settings.notDownloaded")}</span>
            {/if}
          </div>
        </button>
        {#if model.downloaded}
          <button
            onclick={() => void store.uninstallVad(model.id)}
            disabled={store.downloadingVadId !== null}
            class="shrink-0 px-3 py-1.5 rounded-lg text-xs font-bold text-red-300 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 transition-colors cursor-pointer disabled:opacity-50"
          >
            {t("settings.uninstall")}
          </button>
        {:else}
          <button
            onclick={() => void store.downloadVad(model.id)}
            disabled={store.isDownloading || store.downloadingVadId !== null}
            class="shrink-0 px-3 py-1.5 rounded-lg text-xs font-bold text-white bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 transition-colors cursor-pointer"
          >
            {t("settings.download")}
          </button>
        {/if}
      </div>
    {/each}

    <!-- Custom VAD model: arbitrary local .bin, bypasses the table above -->
    <div class="p-4 rounded-xl border flex items-center justify-between gap-4 {store.vadSelection.customPath
      ? 'bg-emerald-500/10 border-emerald-500/25'
      : 'bg-white/5 border-white/10'}">
      <div class="min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <span class="text-sm font-bold text-white">{t("settings.whisper.vadCustomLabel")}</span>
          {#if store.vadSelection.customPath}
            {#if store.vadCustomValid}
              <span class="text-[9px] font-bold text-emerald-400 uppercase tracking-wide">{t("settings.whisper.vadCustomActive")}</span>
            {:else}
              <span class="text-[9px] text-red-400">{t("settings.whisper.vadCustomInvalid")}</span>
            {/if}
          {/if}
        </div>
        {#if store.vadSelection.customPath}
          <p class="text-xs text-gray-400 mt-1 truncate" title={store.vadSelection.customPath}>{store.vadSelection.customPath}</p>
        {/if}
      </div>
      <div class="shrink-0 flex items-center gap-2">
        <button
          onclick={() => void store.pickCustomVad()}
          class="px-3 py-1.5 rounded-lg text-xs font-bold text-white bg-cyan-600 hover:bg-cyan-500 transition-colors cursor-pointer"
        >
          {t("settings.whisper.vadCustomPick")}
        </button>
        {#if store.vadSelection.customPath}
          <button
            onclick={() => store.clearCustomVad()}
            class="px-3 py-1.5 rounded-lg text-xs font-bold text-red-300 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 transition-colors cursor-pointer"
          >
            {t("settings.remove")}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
{/if}

{#if store.contextMenu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50"
    onmousedown={() => store.closeContextMenu()}
    oncontextmenu={(e) => { e.preventDefault(); store.closeContextMenu(); }}
  >
    <div
      class="absolute bg-gray-900/98 border border-white/10 rounded-xl shadow-2xl py-1 min-w-[190px] animate-fade-in overflow-hidden"
      style="left: {store.contextMenu.x}px; top: {store.contextMenu.y}px;"
      onmousedown={(e) => e.stopPropagation()}
    >
      <div class="px-3 py-1.5 border-b border-white/5 bg-white/5 mb-1">
        <span class="text-xs font-bold text-gray-400 uppercase tracking-wide">
          {store.contextMenu.kind === "model" ? `Whisper: ${store.contextMenu.modelId}` : "Whisper"}
        </span>
      </div>
      {#if store.contextMenu.kind === "panel"}
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center gap-2 transition-colors"
          onclick={() => {
            void store.refreshModels();
            store.closeContextMenu();
          }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9M20 20v-5h-.581m0 0a8.003 8.003 0 01-15.357-2" /></svg>
          {t("settings.refreshStatus")}
        </button>
        {#if store.whisperModels.some((model) => !model.downloaded)}
          <button
            class="w-full text-left px-4 py-2 text-sm text-cyan-300 hover:bg-cyan-500/10 hover:text-cyan-200 flex items-center gap-2 transition-colors"
            onclick={() => {
              const nextModel = store.whisperModels.find((model) => !model.downloaded);
              if (nextModel) void store.downloadModel(nextModel.id, true);
              store.closeContextMenu();
            }}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
            {t("settings.downloadNext")}
          </button>
        {/if}
      {:else if store.contextMenu.downloaded && store.contextMenu.modelId}
        <button
          class="w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-white/10 hover:text-white flex items-center gap-2 transition-colors"
          onclick={() => {
            if (store.contextMenu?.modelId) store.setDefaultWhisperModel(store.contextMenu.modelId);
            store.closeContextMenu();
          }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
          {t("settings.setAsDefault")}
        </button>
        <button
          class="w-full text-left px-4 py-2 text-sm text-red-400 hover:bg-red-500/10 hover:text-red-300 flex items-center gap-2 transition-colors"
          onclick={() => {
            if (store.contextMenu?.modelId) void store.uninstallModel(store.contextMenu.modelId);
            store.closeContextMenu();
          }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
          {t("settings.remove")}
        </button>
      {:else if store.contextMenu.modelId}
        <button
          class="w-full text-left px-4 py-2 text-sm text-cyan-300 hover:bg-cyan-500/10 hover:text-cyan-200 flex items-center gap-2 transition-colors"
          onclick={() => {
            if (store.contextMenu?.modelId) void store.downloadModel(store.contextMenu.modelId, true);
            store.closeContextMenu();
          }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
          {t("settings.downloadAndSet")}
        </button>
      {/if}
    </div>
  </div>
{/if}

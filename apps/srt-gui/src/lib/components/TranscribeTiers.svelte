<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { locale } from "$lib/i18n";
  import ProviderIcon from "$lib/components/ProviderIcon.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import TierList from "$lib/components/TierList.svelte";
  import { transcribeProviders } from "$lib/config/transcribeProviders";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "$lib/config/apiKeys";
  import { newTierId, newTierEntryId } from "$lib/config/translationTiers";
  import {
    loadTranscribeTiers,
    saveTranscribeTiers,
    type TranscribeTier,
    type TranscribeTierEntry,
    TRANSCRIBE_TIERS_UPDATED_EVENT,
  } from "$lib/config/transcribeTiers";
  import { providers } from "$lib/config/llmProviders";
  import { transcribeListModels } from "$lib/services/transcribe";

  let t = $derived($locale);

  let tiers = $state<TranscribeTier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);
  let whisperModels = $state<({ id: string; name: string; size: string; speed: string; downloaded: boolean })[]>([
    { id: "tiny", name: "Tiny", size: "~75MB", speed: "~32x", downloaded: false },
    { id: "base", name: "Base", size: "~150MB", speed: "~16x", downloaded: false },
    { id: "small", name: "Small", size: "~500MB", speed: "~6x", downloaded: false },
    { id: "medium", name: "Medium", size: "~1.5GB", speed: "~2x", downloaded: false },
    { id: "large", name: "Large", size: "~3GB", speed: "~1x", downloaded: false },
  ]);

  let downloadingModelId = $state<string | null>(null);
  let downloadProgress = $state<number>(0);

  const WHISPER_PROVIDERS = ["groq", "openai", "deepgram", "assemblyai", "custom_whisper", "local_whisper"];
  function isWhisperKey(k: ApiKeyConfig): boolean {
    return WHISPER_PROVIDERS.includes(k.apiType);
  }

  function refreshKeys() {
    let keys = loadAndValidateApiKeys();
    if (!keys.some((k) => k.id === "local_whisper")) {
      keys.unshift({
        id: "local_whisper",
        name: "Local Whisper",
        apiType: "local_whisper",
        apiKey: "local-not-needed",
        apiUrl: "",
        isValid: true
      });
    } else {
      const idx = keys.findIndex((k) => k.id === "local_whisper");
      if (idx !== -1) {
        const localKey = keys[idx];
        localKey.name = "Local Whisper";
        keys.splice(idx, 1);
        keys.unshift(localKey);
      }
    }
    apiKeys = keys;
  }

  async function refreshWhisperModels() {
    try {
      const models = await transcribeListModels();
      if (models && Array.isArray(models)) {
        whisperModels = models;
      }
    } catch {
      /* ignore */
    }
  }

  function persist() {
    saveTranscribeTiers(tiers);
  }

  function syncTiers() {
    tiers = loadTranscribeTiers();
  }

  onMount(() => {
    const handleModelsUpdated = (e: Event) => {
      const customEvent = e as CustomEvent<{ models: any[] }>;
      if (customEvent.detail?.models) {
        whisperModels = customEvent.detail.models;
      }
    };

    const handleDownloadProgress = (e: Event) => {
      const customEvent = e as CustomEvent<{ modelId: string | null; progress: number }>;
      downloadingModelId = customEvent.detail?.modelId || null;
      downloadProgress = customEvent.detail?.progress || 0;
    };

    tiers = loadTranscribeTiers();
    refreshKeys();
    void refreshWhisperModels();
    window.addEventListener("apikeys-updated", refreshKeys);
    window.addEventListener("apikeys-updated", refreshWhisperModels);
    window.addEventListener("vesta:transcribe-cloud-updated", refreshWhisperModels);
    window.addEventListener("vesta-whisper-models-updated", handleModelsUpdated);
    window.addEventListener("vesta-whisper-download-progress", handleDownloadProgress);
    window.addEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTiers);

    return () => {
      window.removeEventListener("apikeys-updated", refreshKeys);
      window.removeEventListener("apikeys-updated", refreshWhisperModels);
      window.removeEventListener("vesta:transcribe-cloud-updated", refreshWhisperModels);
      window.removeEventListener("vesta-whisper-models-updated", handleModelsUpdated);
      window.removeEventListener("vesta-whisper-download-progress", handleDownloadProgress);
      window.removeEventListener(TRANSCRIBE_TIERS_UPDATED_EVENT, syncTiers);
    };
  });

  // ─── Mutations ──────────────────────────────────────────────────────────────

  function addTier() {
    tiers = [...tiers, { id: newTierId(), entries: [] }];
    persist();
  }

  export function triggerAddTier() {
    addTier();
  }

  function removeTier(index: number) {
    tiers = tiers.filter((_, i) => i !== index);
    persist();
  }

  function moveTier(index: number, dir: -1 | 1) {
    const target = index + dir;
    if (target < 0 || target >= tiers.length) return;
    const copy = [...tiers];
    [copy[index], copy[target]] = [copy[target], copy[index]];
    tiers = copy;
    persist();
  }

  function getTranscribeModelsForProvider(provider: string) {
    if (provider === "local" || provider === "local_whisper") {
      return [
        { id: "tiny", name: "Tiny" },
        { id: "base", name: "Base", recommended: true },
        { id: "small", name: "Small" },
        { id: "medium", name: "Medium" },
        { id: "large", name: "Large" },
      ];
    }
    if (provider === "custom" || provider === "custom_whisper") {
      return [];
    }
    return transcribeProviders[provider]?.models || [];
  }

  function defaultModelFor(provider: string): string {
    const models = getTranscribeModelsForProvider(provider);
    return models.find((m) => m.recommended)?.id || models[0]?.id || "";
  }

  function addEntry(tierIndex: number) {
    const whisperKeys = apiKeys.filter(isWhisperKey);
    const firstKey = whisperKeys.find((k) => k.apiType !== "custom_whisper" && k.apiType !== "local_whisper") || whisperKeys[0];
    const provider = firstKey ? firstKey.apiType : "local_whisper";
    const entry: TranscribeTierEntry = {
      id: newTierEntryId(),
      provider,
      apiKeyId: firstKey?.id || "",
      model: defaultModelFor(provider),
    };
    tiers = tiers.map((tier, i) =>
      i === tierIndex ? { ...tier, entries: [...tier.entries, entry] } : tier,
    );
    persist();
  }

  function removeEntry(tierIndex: number, entryIndex: number) {
    tiers = tiers.map((tier, i) =>
      i === tierIndex
        ? { ...tier, entries: tier.entries.filter((_, j) => j !== entryIndex) }
        : tier,
    );
    persist();
  }

  function onKeyChange(tierIndex: number, entryIndex: number, keyId: string) {
    const key = apiKeys.find((k) => k.id === keyId);
    const provider = key?.apiType || "local_whisper";
    const targetEntry = tiers[tierIndex]?.entries[entryIndex];
    if (targetEntry) setCustomMode(targetEntry.id, false);
    tiers = tiers.map((tier, i) => {
      if (i !== tierIndex) return tier;
      return {
        ...tier,
        entries: tier.entries.map((e, j) => {
          if (j !== entryIndex) return e;
          const stillValid = getTranscribeModelsForProvider(provider).some((m) => m.id === e.model);
          return {
            ...e,
            apiKeyId: keyId,
            provider,
            model: stillValid ? e.model : defaultModelFor(provider),
          };
        }),
      };
    });
    persist();
  }

  function onModelChange(tierIndex: number, entryIndex: number, model: string) {
    tiers = tiers.map((tier, i) =>
      i === tierIndex
        ? {
            ...tier,
            entries: tier.entries.map((e, j) =>
              j === entryIndex ? { ...e, model } : e,
            ),
          }
        : tier,
    );
    persist();
  }

  function onNumberChange(
    tierIndex: number,
    entryIndex: number,
    field: "rpm" | "maxRequests",
    raw: string,
  ) {
    const n = parseInt(raw, 10);
    const value = Number.isFinite(n) && n > 0 ? n : undefined;
    tiers = tiers.map((tier, i) =>
      i === tierIndex
        ? {
            ...tier,
            entries: tier.entries.map((e, j) =>
              j === entryIndex ? { ...e, [field]: value } : e,
            ),
          }
        : tier,
    );
    persist();
  }

  // Label per il dropdown delle key
  function keyLabel(key: ApiKeyConfig): string {
    if (key.apiType === "local_whisper") return "Local Whisper";
    const provName = transcribeProviders[key.apiType]?.name || key.apiType;
    return key.name && key.name !== provName ? `${provName} · ${key.name}` : provName;
  }

  function entryModels(entry: TranscribeTierEntry): { id: string; name: string; recommended?: boolean }[] {
    return getTranscribeModelsForProvider(entry.provider);
  }

  let customModeIds = $state<Set<string>>(new Set());

  function setCustomMode(id: string, on: boolean) {
    const next = new Set(customModeIds);
    if (on) next.add(id);
    else next.delete(id);
    customModeIds = next;
  }

  function isCustomModel(entry: TranscribeTierEntry): boolean {
    if (customModeIds.has(entry.id)) return true;
    const models = entryModels(entry);
    if (models.length === 0) return true;
    if (!entry.model) return false;
    return !models.some((m) => m.id === entry.model);
  }
</script>

{#snippet noKeysWarning()}
  {#if apiKeys.filter(isWhisperKey).length === 0}
    <div class="rounded-lg border border-amber-400/20 bg-amber-500/10 px-3 py-2.5 text-xs text-amber-100">
      {t("tiers.noKeys") || "Nessuna API key configurata. Aggiungi prima una chiave sopra, quindi selezionala qui."}
    </div>
  {/if}
{/snippet}

<TierList
  {tiers}
  emptyMessage={t("tiers.empty") || "Nessun tier. Aggiungi un tier per iniziare."}
  priorityLabel={(i) => (i === 0 ? (t("tiers.highestPriority") || "priorità massima") : (t("tiers.fallback") || "fallback"))}
  moveUpLabel={t("tiers.moveUp")}
  moveDownLabel={t("tiers.moveDown")}
  removeTierLabel={t("tiers.removeTier")}
  onMoveTier={moveTier}
  onRemoveTier={removeTier}
  warning={noKeysWarning}
>
  {#snippet tierBody(tier, tierIndex)}
              {#if tier.entries.length === 0}
                <p class="text-xs text-gray-500 px-1 py-1">{t("tiers.tierEmpty") || "Tier vuoto — aggiungi almeno un endpoint."}</p>
              {/if}

              {#each tier.entries as entry, entryIndex (entry.id)}
                {@const isDownloaded = (entry.provider === "local" || entry.provider === "local_whisper") ? (whisperModels.find(m => m.id === entry.model)?.downloaded ?? false) : true}
                <!-- Flat Endpoint Card -->
                <div class="rounded-xl border border-white/5 bg-white/[0.02] p-4.5 space-y-3.5 shadow-sm">
                  <!-- Header row: Provider selection + Remove button -->
                  <div class="flex items-center justify-between gap-2 border-b border-white/5 pb-2.5">
                    <div class="flex items-center gap-2">
                      <ProviderIcon provider={entry.provider} size="w-5.5 h-5.5" glyph="w-3 h-3" rounded="rounded-md" />
                      <span class="text-xs font-semibold text-gray-200">{providers[entry.provider]?.name || entry.provider}</span>
                    </div>
                    <button
                      type="button"
                      onclick={() => removeEntry(tierIndex, entryIndex)}
                      class="px-2.5 py-1 rounded-lg text-xs text-red-400 bg-red-500/5 border border-red-500/10 hover:text-white hover:bg-red-500/20 hover:border-red-500/30 transition cursor-pointer font-semibold"
                    >
                      {t("tiers.removeEntry") || "Rimuovi"}
                    </button>
                  </div>

                  <!-- Fields grid (API Key & Model) -->
                  <div class="grid grid-cols-1 sm:grid-cols-12 gap-3 items-end">
                    <!-- API key -->
                    <div class="sm:col-span-5">
                      <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wide mb-1">
                        {(entry.provider === "local" || entry.provider === "local_whisper") ? (t("translate.provider") || "Provider") : (t("tiers.apiKey") || "API Key")}
                      </span>
                      <SearchableSelect
                        options={apiKeys.filter(isWhisperKey).length === 0
                          ? [{ value: "", label: t("tiers.noKeysOption") || "Nessuna key" }]
                          : apiKeys.filter(isWhisperKey).map((key) => ({ value: key.id, label: keyLabel(key), provider: key.apiType }))}
                        value={entry.apiKeyId}
                        onchange={(val) => onKeyChange(tierIndex, entryIndex, val)}
                        placeholder={t("tiers.noKeysOption") || "Nessuna key"}
                      />
                    </div>

                    <!-- Model -->
                    <div class="sm:col-span-7">
                      <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wide mb-1">{t("tiers.model") || "Modello"}</span>
                      <div class="flex items-center gap-2">
                        <div class="flex-1 min-w-0">
                          <SearchableSelect
                            options={[
                              ...entryModels(entry).map((m) => {
                                const isLocal = entry.provider === "local" || entry.provider === "local_whisper";
                                const modelDetail = isLocal ? whisperModels.find(wm => wm.id === m.id) : null;
                                const isDownloaded = modelDetail ? modelDetail.downloaded : false;
                                
                                const readyIcon = `<svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>`;
                                const notDownloadedIcon = `<svg class="w-4 h-4 text-red-500/80" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1" /><path stroke-linecap="round" stroke-linejoin="round" d="M12 4v8m0 0l-3-3m3 3l3-3" /><path stroke-linecap="round" stroke-linejoin="round" d="M3 3l18 18" /></svg>`;
                                
                                return {
                                  value: m.id,
                                  label: m.name,
                                  icon: isLocal ? (isDownloaded ? readyIcon : notDownloadedIcon) : ""
                                };
                              }),
                              ...((entry.provider === "local" || entry.provider === "local_whisper") ? [] : [{ value: "__custom__", label: t("tiers.customModel") || "Modello personalizzato..." }])
                            ]}
                            value={isCustomModel(entry) ? "__custom__" : entry.model}
                            onchange={(v) => {
                              if (v === "__custom__") {
                                setCustomMode(entry.id, true);
                                onModelChange(tierIndex, entryIndex, "");
                              } else {
                                setCustomMode(entry.id, false);
                                onModelChange(tierIndex, entryIndex, v);
                              }
                            }}
                            placeholder={t("tiers.model") || "Modello"}
                          />
                        </div>

                        {#if entry.provider === "local" || entry.provider === "local_whisper"}
                          {@const modelDetail = whisperModels.find(wm => wm.id === entry.model)}
                          {@const isDownloaded = modelDetail ? modelDetail.downloaded : false}
                          <div class="flex gap-1.5 shrink-0">
                            <!-- Download button -->
                            <button
                              type="button"
                              onclick={() => window.dispatchEvent(new CustomEvent("vesta-download-whisper-model", { detail: { modelId: entry.model } }))}
                              disabled={isDownloaded || downloadingModelId === entry.model}
                              class="px-3 py-2.5 rounded-lg text-xs font-bold transition-all duration-200 cursor-pointer
                                {isDownloaded
                                  ? 'bg-white/5 border border-white/10 text-gray-500 cursor-not-allowed'
                                  : 'bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-500/20'}"
                            >
                              {t("settings.download") || "Scarica"}
                            </button>

                            <!-- Uninstall button -->
                            <button
                              type="button"
                              onclick={() => window.dispatchEvent(new CustomEvent("vesta-uninstall-whisper-model", { detail: { modelId: entry.model } }))}
                              disabled={!isDownloaded || downloadingModelId === entry.model}
                              class="px-3 py-2.5 rounded-lg text-xs font-bold transition-all duration-200 cursor-pointer
                                {!isDownloaded
                                  ? 'bg-white/5 border border-white/10 text-gray-500 cursor-not-allowed'
                                  : 'bg-red-500/10 border border-red-500/20 text-red-400 hover:bg-red-500/20 hover:border-red-500/30'}"
                            >
                              {t("settings.uninstall") || "Disinstalla"}
                            </button>
                          </div>
                        {/if}
                      </div>
                    </div>
                  </div>

                  {#if isCustomModel(entry)}
                    <input
                      type="text"
                      value={entry.model}
                      oninput={(e) => onModelChange(tierIndex, entryIndex, (e.currentTarget as HTMLInputElement).value)}
                      placeholder={t("tiers.customModelPlaceholder") || "exact model id"}
                      class="input-modern w-full text-xs font-mono mt-1"
                    />
                  {/if}

                  {#if (entry.provider === "local" || entry.provider === "local_whisper") && downloadingModelId === entry.model}
                    <div class="mt-2.5 flex items-center gap-3 p-2.5 rounded-lg border border-indigo-500/20 bg-indigo-500/5">
                      <span class="text-xs text-indigo-300 font-medium whitespace-nowrap">{t("transcribe.downloadProgress", { progress: downloadProgress })}</span>
                      <div class="flex-1 bg-white/10 rounded-full h-1.5 overflow-hidden">
                        <div class="bg-indigo-400 h-1.5 transition-all duration-150" style="width: {downloadProgress}%"></div>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}

              <!-- Add Endpoint Inline Button -->
              <div class="pt-1 select-none">
                <button
                  type="button"
                  onclick={() => addEntry(tierIndex)}
                  class="inline-flex items-center gap-1.5 py-1.5 px-3 rounded-lg border border-indigo-500/20 text-xs font-semibold text-indigo-400 hover:text-indigo-300 hover:bg-indigo-500/10 hover:border-indigo-500/40 transition cursor-pointer"
                >
                  + {t("tiers.addEntry") || "Aggiungi endpoint"}
                </button>
              </div>
  {/snippet}
</TierList>

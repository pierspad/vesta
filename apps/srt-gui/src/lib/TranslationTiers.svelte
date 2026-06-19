<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { locale } from "./i18n";
  import ProviderIcon from "./ProviderIcon.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import { discoverModels, type DiscoveredModel } from "./modelDiscovery";
  import {
    providers,
    getModelsForProvider,
    loadAndValidateApiKeys,
    loadTiers,
    saveTiers,
    newTierId,
    newTierEntryId,
    TIERS_UPDATED_EVENT,
    type ApiKeyConfig,
    type Tier,
    type TierEntry,
  } from "./models";

  let t = $derived($locale);

  let tiers = $state<Tier[]>([]);
  let apiKeys = $state<ApiKeyConfig[]>([]);

  // Provider che richiedono una API key (per i suggerimenti su come aggiungerla).
  function keysForProvider(provider: string): ApiKeyConfig[] {
    return apiKeys.filter((k) => k.apiType === provider);
  }

  let localOffline = $state(false);

  function refreshKeys() {
    let keys = loadAndValidateApiKeys();
    if (!keys.some((k) => k.id === "local")) {
      keys.unshift({
        id: "local",
        name: "Local LLM (Open API)",
        apiType: "local",
        apiKey: "local-not-needed",
        apiUrl: localStorage.getItem("vesta-local-server-url") || "http://localhost:11434/v1",
        isValid: true
      });
    } else {
      const idx = keys.findIndex((k) => k.id === "local");
      if (idx !== -1) {
        const localKey = keys[idx];
        localKey.name = "Local LLM (Open API)";
        keys.splice(idx, 1);
        keys.unshift(localKey);
      }
    }
    apiKeys = keys;
  }

  function persist() {
    saveTiers(tiers);
  }

  // ─── Model auto-discovery ─────────────────────────────────────────────────────
  // Per ogni (provider, key) interroghiamo a runtime l'endpoint dei modelli, così
  // nuovi modelli compaiono senza dover aggiornare l'app. Cache in memoria + in
  // localStorage (TTL 24h) per non rifare la chiamata a ogni render.

  let discovered = $state<Map<string, DiscoveredModel[]>>(new Map());
  const inflight = new Set<string>();
  const DISCOVERY_TTL_MS = 24 * 60 * 60 * 1000;

  function discoveryKey(provider: string, apiKeyId: string): string {
    return `${provider}::${apiKeyId}`;
  }

  async function ensureModels(provider: string, apiKeyId: string, force = false) {
    const cacheId = discoveryKey(provider, apiKeyId);
    if (inflight.has(cacheId)) return;
    if (!force && discovered.has(cacheId) && provider !== "local") return;

    const key = apiKeys.find((k) => k.id === apiKeyId);
    const needsKey = provider !== "local" && provider !== "custom";
    if (needsKey && !key?.apiKey?.trim()) return;

    const lsKey = `vesta-tier-models-${cacheId}`;
    if (!force && provider !== "local") {
      try {
        const raw = localStorage.getItem(lsKey);
        if (raw) {
          const parsed = JSON.parse(raw);
          if (parsed && Array.isArray(parsed.models) && Date.now() - parsed.ts < DISCOVERY_TTL_MS) {
            discovered.set(cacheId, parsed.models);
            discovered = new Map(discovered);
            return;
          }
        }
      } catch {
        /* ignore */
      }
    }

    inflight.add(cacheId);
    try {
      const apiUrl = key?.apiUrl?.trim() || providers[provider]?.defaultApiUrl || "";
      const models = await discoverModels(provider, key?.apiKey || "", apiUrl);
      discovered.set(cacheId, models);
      discovered = new Map(discovered);
      if (provider === "local") {
        localOffline = false;
      }
      try {
        localStorage.setItem(lsKey, JSON.stringify({ ts: Date.now(), models }));
      } catch {
        /* ignore quota */
      }
    } catch {
      // Endpoint non raggiungibile o key non valida: si resta sui modelli curati.
      if (provider === "local") {
        localOffline = true;
      }
    } finally {
      inflight.delete(cacheId);
    }
  }

  function refreshEntryModels(entry: TierEntry) {
    void ensureModels(entry.provider, entry.apiKeyId, true);
  }

  // Avvia la discovery per ogni entry presente (mount, cambio key, nuova entry).
  $effect(() => {
    for (const tier of tiers) {
      for (const e of tier.entries) {
        void ensureModels(e.provider, e.apiKeyId);
      }
    }
  });

  function syncTiers() {
    tiers = loadTiers();
  }

  onMount(() => {
    tiers = loadTiers();
    refreshKeys();
    window.addEventListener("apikeys-updated", refreshKeys);
    window.addEventListener(TIERS_UPDATED_EVENT, syncTiers);
  });

  onDestroy(() => {
    window.removeEventListener("apikeys-updated", refreshKeys);
    window.removeEventListener(TIERS_UPDATED_EVENT, syncTiers);
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

  function defaultModelFor(provider: string): string {
    const models = getModelsForProvider(provider);
    return models.find((m) => m.recommended)?.id || models[0]?.id || "";
  }

  function addEntry(tierIndex: number) {
    const firstKey = apiKeys.find((k) => k.apiType !== "custom") || apiKeys[0];
    const provider = (firstKey?.apiType as TierEntry["provider"]) || "google";
    const entry: TierEntry = {
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

  // Aggiorna l'API key di una entry e riallinea provider + modello.
  function onKeyChange(tierIndex: number, entryIndex: number, keyId: string) {
    const key = apiKeys.find((k) => k.id === keyId);
    const provider = (key?.apiType as TierEntry["provider"]) || "google";
    // La scelta del modello custom non sopravvive al cambio provider.
    const targetEntry = tiers[tierIndex]?.entries[entryIndex];
    if (targetEntry) setCustomMode(targetEntry.id, false);
    tiers = tiers.map((tier, i) => {
      if (i !== tierIndex) return tier;
      return {
        ...tier,
        entries: tier.entries.map((e, j) => {
          if (j !== entryIndex) return e;
          const stillValid = getModelsForProvider(provider).some((m) => m.id === e.model);
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

  function keyLabel(key: ApiKeyConfig): string {
    if (key.apiType === "local") return "Local LLM (Open API)";
    const provName = providers[key.apiType]?.name || key.apiType;
    return key.name && key.name !== provName ? `${provName} · ${key.name}` : provName;
  }

  // Modelli mostrati per una entry: curati (con ★ consigliati) uniti a quelli
  // scoperti a runtime dall'endpoint del provider (deduplicati per id).
  function entryModels(entry: TierEntry): { id: string; name: string; recommended?: boolean }[] {
    const merged = new Map<string, { id: string; name: string; recommended?: boolean }>();
    for (const m of getModelsForProvider(entry.provider)) {
      merged.set(m.id, { id: m.id, name: m.name, recommended: m.recommended });
    }
    const disc = discovered.get(discoveryKey(entry.provider, entry.apiKeyId)) || [];
    for (const d of disc) {
      if (!merged.has(d.id)) merged.set(d.id, { id: d.id, name: d.name });
    }
    return [...merged.values()];
  }

  // Entry per cui l'utente ha scelto esplicitamente "modello personalizzato".
  let customModeIds = $state<Set<string>>(new Set());

  function setCustomMode(id: string, on: boolean) {
    const next = new Set(customModeIds);
    if (on) next.add(id);
    else next.delete(id);
    customModeIds = next;
  }

  // Una entry usa il modello custom (free text) quando: l'utente l'ha scelto,
  // oppure il provider non ha modelli predefiniti, oppure il modello salvato
  // non è tra quelli predefiniti.
  function isCustomModel(entry: TierEntry): boolean {
    if (customModeIds.has(entry.id)) return true;
    const models = entryModels(entry);
    if (models.length === 0) return true;
    if (!entry.model) return false;
    return !models.some((m) => m.id === entry.model);
  }

  let totalEntries = $derived(tiers.reduce((sum, t) => sum + t.entries.length, 0));
  let collapsedTiers = $state<Set<string>>(new Set());

  function toggleTier(tierId: string) {
    const next = new Set(collapsedTiers);
    if (next.has(tierId)) {
      next.delete(tierId);
    } else {
      next.add(tierId);
    }
    collapsedTiers = next;
  }

  const LLM_PROVIDERS = ["google", "groq", "openai", "openrouter", "mistral", "github", "nvidia", "custom", "local"];
</script>

<div class="space-y-4">
  {#if apiKeys.length === 0}
    <div class="rounded-lg border border-amber-400/20 bg-amber-500/10 px-3 py-2.5 text-xs text-amber-100">
      {t("tiers.noKeys")}
    </div>
  {/if}

  {#if tiers.length === 0}
    <div class="rounded-lg border border-white/10 bg-white/5 px-4 py-6 text-center text-sm text-gray-400">
      {t("tiers.empty")}
    </div>
  {:else}
    <div class="space-y-6">
      {#each tiers as tier, tierIndex (tier.id)}
        <div class="border-b border-white/5 pb-6 last:border-b-0 last:pb-0">
          <!-- Tier Accordion Header -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            role="button"
            tabindex="0"
            onclick={() => toggleTier(tier.id)}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleTier(tier.id); } }}
            class="flex items-center justify-between gap-2 py-3 cursor-pointer group select-none outline-none"
          >
            <div class="flex items-center gap-3">
              <!-- Chevron indicator -->
              <span class="text-gray-500 group-hover:text-white transition-colors duration-150 flex-shrink-0">
                {#if collapsedTiers.has(tier.id)}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" /></svg>
                {:else}
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" /></svg>
                {/if}
              </span>

              <!-- Badge with Order Number -->
              <span class="inline-flex items-center justify-center w-6 h-6 rounded-md bg-indigo-500/15 text-indigo-300 text-xs font-bold flex-shrink-0">
                {tierIndex + 1}
              </span>

              <!-- Title & Priority sub-label -->
              <div class="flex flex-col">
                <span class="text-sm font-bold text-white group-hover:text-indigo-200 transition-colors">
                  {t("tiers.tier")} {tierIndex + 1}
                </span>
                <span class="text-[10px] text-gray-500 font-medium mt-0.5 uppercase tracking-wider">
                  {tierIndex === 0 ? t("tiers.highestPriority") : t("tiers.fallback")}
                </span>
              </div>
            </div>

            <!-- Tier Actions (Control Buttons) -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div class="flex items-center gap-1" onclick={(e) => e.stopPropagation()}>
              <button
                type="button"
                onclick={() => moveTier(tierIndex, -1)}
                disabled={tierIndex === 0}
                title={t("tiers.moveUp")}
                class="p-1.5 rounded-lg text-indigo-400 bg-indigo-500/5 border border-indigo-500/10 hover:text-white hover:bg-indigo-500/20 hover:border-indigo-500/30 disabled:opacity-20 disabled:hover:bg-transparent disabled:border-transparent transition cursor-pointer"
                aria-label={t("tiers.moveUp")}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 15l7-7 7 7" /></svg>
              </button>
              <button
                type="button"
                onclick={() => moveTier(tierIndex, 1)}
                disabled={tierIndex === tiers.length - 1}
                title={t("tiers.moveDown")}
                class="p-1.5 rounded-lg text-indigo-400 bg-indigo-500/5 border border-indigo-500/10 hover:text-white hover:bg-indigo-500/20 hover:border-indigo-500/30 disabled:opacity-20 disabled:hover:bg-transparent disabled:border-transparent transition cursor-pointer"
                aria-label={t("tiers.moveDown")}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" /></svg>
              </button>
              <button
                type="button"
                onclick={() => removeTier(tierIndex)}
                title={t("tiers.removeTier")}
                class="p-1.5 rounded-lg text-red-400 bg-red-500/5 border border-red-500/10 hover:text-white hover:bg-red-500/20 hover:border-red-500/30 transition cursor-pointer"
                aria-label={t("tiers.removeTier")}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
              </button>
            </div>
          </div>

          <!-- Tier Content (Endpoints list) -->
          {#if !collapsedTiers.has(tier.id)}
            <div class="pl-6 pr-2 py-2 space-y-4">
              {#if tier.entries.length === 0}
                <p class="text-xs text-gray-500 px-1 py-1">{t("tiers.tierEmpty")}</p>
              {/if}

              {#each tier.entries as entry, entryIndex (entry.id)}
                <!-- Flat Endpoint Card -->
                <div class="rounded-xl border p-4.5 space-y-3.5 shadow-sm transition-all duration-200
                  {entry.provider === 'local' && localOffline
                    ? 'border-amber-500/40 bg-amber-500/[0.02] shadow-[0_0_12px_rgba(245,158,11,0.05)]'
                    : 'border-white/5 bg-white/[0.02]'}"
                >
                  <!-- Header row: Provider + Remove button -->
                  <div class="flex items-center justify-between gap-2 border-b border-white/5 pb-2.5">
                    <div class="flex items-center gap-2">
                      <ProviderIcon provider={entry.provider} size="w-5.5 h-5.5" glyph="w-3 h-3" rounded="rounded-md" />
                      <span class="text-xs font-semibold text-gray-200">{providers[entry.provider]?.name || entry.provider}</span>
                      {#if entry.provider === "local" && localOffline}
                        <span class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20 text-[9px] font-bold">
                          <span class="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse"></span>
                          Offline
                        </span>
                      {/if}
                    </div>
                    <button
                      type="button"
                      onclick={() => removeEntry(tierIndex, entryIndex)}
                      class="px-2.5 py-1 rounded-lg text-xs text-red-400 bg-red-500/5 border border-red-500/10 hover:text-white hover:bg-red-500/20 hover:border-red-500/30 transition cursor-pointer font-semibold"
                    >
                      {t("tiers.removeEntry")}
                    </button>
                  </div>

                  <!-- Fields grid (API Key & Model) -->
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 items-end">
                    <!-- API key -->
                    <div>
                      <span class="block text-[10px] font-bold text-gray-500 uppercase tracking-wide mb-1">
                        {entry.provider === "local" ? (t("translate.provider") || "Provider") : (t("tiers.apiKey") || "API Key")}
                      </span>
                      <SearchableSelect
                        options={apiKeys.filter((k) => LLM_PROVIDERS.includes(k.apiType)).length === 0
                          ? [{ value: "", label: t("tiers.noKeysOption") }]
                          : apiKeys.filter((k) => LLM_PROVIDERS.includes(k.apiType)).map((key) => ({ value: key.id, label: keyLabel(key), provider: key.apiType }))}
                        value={entry.apiKeyId}
                        onchange={(val) => onKeyChange(tierIndex, entryIndex, val)}
                        placeholder={t("tiers.noKeysOption")}
                      />
                    </div>

                    <!-- Model -->
                    <div>
                      <div class="flex items-center justify-between mb-1">
                        <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("tiers.model")}</span>
                        <button
                          type="button"
                          onclick={() => refreshEntryModels(entry)}
                          title={t("tiers.refreshModels")}
                          aria-label={t("tiers.refreshModels")}
                          class="p-0.5 text-gray-500 hover:text-indigo-300 transition cursor-pointer"
                        >
                          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
                        </button>
                      </div>
                      <SearchableSelect
                        options={[
                          ...entryModels(entry).map((m) => ({
                            value: m.id,
                            label: `${m.name}${m.recommended ? " ★" : ""}`
                          })),
                          { value: "__custom__", label: t("tiers.customModel") }
                        ]}
                        value={isCustomModel(entry) ? "__custom__" : entry.model}
                        onfocus={() => ensureModels(entry.provider, entry.apiKeyId, true)}
                        onchange={(v) => {
                          if (v === "__custom__") {
                            setCustomMode(entry.id, true);
                            onModelChange(tierIndex, entryIndex, "");
                          } else {
                            setCustomMode(entry.id, false);
                            onModelChange(tierIndex, entryIndex, v);
                          }
                        }}
                        placeholder={t("tiers.model")}
                      />
                    </div>
                  </div>

                  {#if isCustomModel(entry)}
                    <input
                      type="text"
                      value={entry.model}
                      oninput={(e) => onModelChange(tierIndex, entryIndex, (e.currentTarget as HTMLInputElement).value)}
                      placeholder={t("tiers.customModelPlaceholder")}
                      class="input-modern w-full text-xs font-mono mt-1"
                    />
                  {/if}

                  <!-- RPM / Budget inline fields -->
                  <div class="flex flex-wrap items-center gap-x-5 gap-y-2 pt-1">
                    <div class="flex items-center gap-2 shrink-0">
                      <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("tiers.rpm")}:</span>
                      <input
                        type="number"
                        min="0"
                        value={entry.rpm ?? ""}
                        oninput={(e) => onNumberChange(tierIndex, entryIndex, "rpm", (e.currentTarget as HTMLInputElement).value)}
                        placeholder={t("tiers.auto")}
                        class="input-modern w-20 text-xs py-1.5 px-2 text-center"
                      />
                    </div>
                    <div class="flex items-center gap-2 shrink-0">
                      <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{t("tiers.budget")}:</span>
                      <input
                        type="number"
                        min="0"
                        value={entry.maxRequests ?? ""}
                        oninput={(e) => onNumberChange(tierIndex, entryIndex, "maxRequests", (e.currentTarget as HTMLInputElement).value)}
                        placeholder={t("tiers.unlimited")}
                        class="input-modern w-20 text-xs py-1.5 px-2 text-center"
                      />
                    </div>
                  </div>
                </div>
              {/each}

              <!-- Add Endpoint Inline Button -->
              <div class="pt-1 select-none">
                <button
                  type="button"
                  onclick={() => addEntry(tierIndex)}
                  class="inline-flex items-center gap-1.5 py-1.5 px-3 rounded-lg border border-indigo-500/20 text-xs font-semibold text-indigo-400 hover:text-indigo-300 hover:bg-indigo-500/10 hover:border-indigo-500/40 transition cursor-pointer"
                >
                  + {t("tiers.addEntry")}
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

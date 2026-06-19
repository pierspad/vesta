<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
  import { onDestroy, onMount } from "svelte";
  import { locale } from "./i18n";
  import {
    detectLanguageCode,
    getLanguageSearchTerms,
    getModelsForProvider,
    languages,
    loadAndValidateApiKeys,
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    providers,
    type ApiKeyConfig,
    type Tier,
    getFileName,
  } from "./models";
  import PathPickerField from "./PathPickerField.svelte";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import { snackbar } from "./snackbarStore.svelte";
  import { aiStore } from "./aiStore.svelte";
  import {
    extractModelsFromPayload,
    fetchModelsFromEndpoint,
    type DiscoveredModel,
  } from "./modelDiscovery";

  /**
   * Generates a smart output path by detecting and replacing language codes
   * in the filename. If the last segment before .srt (delimited by - . or _)
   * is a known language code, it replaces it keeping the same separator.
   * Otherwise, appends .{targetLang} before .srt.
   *
   * Examples:
   *   Detour-en.srt      → Detour-it.srt
   *   movie.eng.srt       → movie.it.srt
   *   sub_fr_720p.srt     → sub_fr_720p.it.srt  (fr is not the LAST segment)
   *   Movie.srt           → Movie.it.srt
   */
  function generateOutputPath(input: string, lang: string): string {
    // Match: (everything)(separator)(segment).srt
    // where separator is one of - . _
    const match = input.match(/^(.+)([\-._])([^\-._]+)\.srt$/i);
    if (match) {
      const [, prefix, separator, lastSegment] = match;
      if (detectLanguageCode(lastSegment)) {
        return `${prefix}${separator}${lang}.srt`;
      }
    }
    // Fallback: append .lang.srt
    return input.replace(/\.srt$/i, `.${lang}.srt`);
  }

  interface Props {
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "language" | "anki") => void;
    active?: boolean;
  }

  let { onGoToSettings, active = true }: Props = $props();

  let t = $derived($locale);

  interface SrtFileInfo {
    path: string;
    subtitle_count: number;
    first_subtitle: string;
    last_subtitle: string;
  }

  interface TierEntryPayload {
    provider: string;
    model: string;
    api_key: string | null;
    api_url: string | null;
    rpm: number | null;
    max_requests: number | null;
  }

  interface TranslateConfig {
    input_path: string;
    output_path: string;
    target_lang: string;
    batch_size: number;
    resume_overlap: number;
    title_context: string | null;
    tiers: TierEntryPayload[][];
  }

  interface TranslateProgressEvent {
    message: string;
    current_batch: number;
    total_batches: number;
    percentage: number;
    eta_seconds: number | null;
  }

  interface TranslateResult {
    success: boolean;
    message: string;
    output_path: string | null;
    translated_count: number;
  }

  interface ModelInfo {
    id: string;
    name: string;
    provider: string;
  }

  const LAST_PROVIDER_KEY = "vesta-translate-last-provider";
  const LAST_MODEL_KEY = "vesta-translate-last-model";
  const LAST_CUSTOM_PROVIDER_KEY = "vesta-translate-last-custom-provider";
  const LAST_CUSTOM_MODEL_KEY = "vesta-translate-last-custom-model";
  const LAST_TARGET_LANGUAGE_KEY = "vesta-translate-target-language";
  const DEFAULT_LLM_PROVIDER_KEY = "vesta-default-llm-provider";
  const DEFAULT_LLM_MODEL_KEY = "vesta-default-llm-model";
  const DEFAULT_LLM_CUSTOM_PROVIDER_KEY = "vesta-default-llm-custom-provider";
  const DEFAULT_TARGET_LANGUAGE_KEY = "vesta-default-target-language";

  function loadStoredValue(key: string): string {
    try {
      return localStorage.getItem(key) || "";
    } catch {
      return "";
    }
  }

  let inputPath = $state("");
  let outputPath = $state("");
  const initialTargetLang = loadStoredValue(LAST_TARGET_LANGUAGE_KEY) || loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY) || "it";
  let targetLang = $state(initialTargetLang);
  let previousTargetLang = initialTargetLang;
  const initialProvider = loadStoredValue(DEFAULT_LLM_PROVIDER_KEY) || loadStoredValue(LAST_PROVIDER_KEY) || "local";
  let selectedProviderFamily = $state(initialProvider);
  let providerConfirmed = $state(true);
  let selectedCustomProviderId = $state(loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY) || loadStoredValue(LAST_CUSTOM_PROVIDER_KEY));

  let localCustomModel = $state(loadStoredValue(LAST_CUSTOM_MODEL_KEY));
  let batchSize = $state(15);
  let resumeOverlap = $state(2);
  let titleContext = $state("");
  let selectedModel = $state(loadStoredValue(DEFAULT_LLM_MODEL_KEY) || loadStoredValue(LAST_MODEL_KEY));

  // Local server URL with persistence
  const LOCAL_SERVER_URL_KEY = "vesta-local-server-url";
  const DEFAULT_LOCAL_URL = "http://localhost:11434/v1";
  let localServerUrl = $state(localStorage.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL);

  // Dynamically fetched models from local/custom server
  let fetchedModels = $state<DiscoveredModel[]>([]);
  let isFetchingModels = $state(false);
  let fetchModelsError = $state("");

  function saveLocalServerUrl(url: string) {
    localServerUrl = url;
    localStorage.setItem(LOCAL_SERVER_URL_KEY, url);
  }

  async function fetchModelsFromServer(baseUrl: string, force = false) {
    // Only cache if it's for the selected custom provider, to avoid mixing URLs.
    // For local, we don't cache since it runs locally and is fast, but we could. Let's just cache custom.
    const cacheKey = selectedProviderFamily === "custom" && selectedCustomProviderId
        ? `vesta-dynamic-models-custom-${selectedCustomProviderId}`
        : null;

    if (!force && cacheKey) {
      const cached = localStorage.getItem(cacheKey);
      if (cached) {
        try {
          const parsed = JSON.parse(cached);
          if (Array.isArray(parsed) && parsed.length > 0) {
            fetchedModels = parsed;
            isFetchingModels = false;
            fetchModelsError = "";
            if (!selectedModel || !parsed.find((m) => m.id === selectedModel)) {
              selectedModel = parsed[0].id;
            }
            return;
          }
        } catch (e) {}
      }
    }

    isFetchingModels = true;
    fetchModelsError = "";
    fetchedModels = [];
    try {
      const models = await fetchModelsFromEndpoint(baseUrl);
      fetchedModels = models;
      if (cacheKey) {
        localStorage.setItem(cacheKey, JSON.stringify(models));
      }
      
      // Auto-select first model if none selected
      if (!selectedModel || !models.find((m) => m.id === selectedModel)) {
        selectedModel = models[0].id;
      }
    } catch (e: any) {
      fetchModelsError = e?.message || "Connection failed";
      fetchedModels = [];
    } finally {
      isFetchingModels = false;
    }
  }

  const batchPresets = [
    { id: "precise", value: 5 },
    { id: "balanced", value: 15 },
    { id: "fast", value: 30 },
    { id: "turbo", value: 50 },
  ] as const;
  let activeBatchPreset = $derived(
    batchPresets.find((p) => p.value === batchSize)?.id ?? null,
  );
  function setBatchPreset(presetId: string) {
    const preset = batchPresets.find((p) => p.id === presetId);
    if (preset) batchSize = preset.value;
  }

  let fileInfo = $state<SrtFileInfo | null>(null);
  let isTranslating = $state(false);
  $effect(() => {
    aiStore.isTranslating = isTranslating;
  });
  let progress = $state<TranslateProgressEvent | null>(null);
  let logs = $state<LogEntry[]>([]);
  let logIdCounter = 0;
  let error = $state<string | null>(null);
  let result = $state<TranslateResult | null>(null);
  let expandedPathField = $state<string | null>(null);

  // Live subtitle preview - array of translated subtitle pairs
  interface SubtitlePair {
    id: number;
    original: string;
    translated: string;
  }
  let translatedPairs = $state<SubtitlePair[]>([]);
  let previewRefreshInterval: ReturnType<typeof setInterval> | null = null;

  let apiKeys = $state<ApiKeyConfig[]>([]);

  // Tier list (priority + failover). Quando presente ha la precedenza sul provider singolo.
  let tiers = $state<Tier[]>([]);
  let useTiers = $derived(tiersHaveUsableEntries(tiers));
  let usableTierCount = $derived(
    tiers.filter((tier) => tier.entries.some((e) => e.model && e.model.trim())).length,
  );

  function refreshTiers() {
    tiers = loadTiers();
  }

  // Costruisce il payload a tier risolvendo le API key referenziate.
  function buildTiersPayload(): TierEntryPayload[][] | null {
    if (!useTiers) return null;
    const out: TierEntryPayload[][] = [];
    for (const tier of tiers) {
      const entries: TierEntryPayload[] = [];
      for (const e of tier.entries) {
        if (!e.model || !e.model.trim()) continue;
        const key = apiKeys.find((k) => k.id === e.apiKeyId);
        const provider = e.provider || key?.apiType || "google";
        const apiKeyVal = key?.apiKey?.trim() || null;
        const apiUrl =
          key?.apiUrl?.trim() || providers[provider]?.defaultApiUrl || null;
        // Le entry remote senza key valida vengono saltate.
        if (!apiKeyVal && provider !== "local" && provider !== "custom") continue;
        entries.push({
          provider,
          model: e.model.trim(),
          api_key: apiKeyVal,
          api_url: apiUrl,
          rpm: e.rpm ?? null,
          max_requests: e.maxRequests ?? null,
        });
      }
      if (entries.length > 0) out.push(entries);
    }
    return out.length > 0 ? out : null;
  }

  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  let unlistenDragDrop: (() => void) | null = null;
  let isDraggingOver = $state(false);

  let canUseFilePanel = $derived(providerConfirmed);

  function ensureProviderSelectedForFiles(): boolean {
    if (providerConfirmed) return true;
    error = "Select a provider before loading files.";
    return false;
  }

  async function handleFileDrop(paths: string[]) {
    if (!ensureProviderSelectedForFiles()) return;
    const srtFile = paths.find(p => p.toLowerCase().endsWith(".srt"));
    if (srtFile) {
      inputPath = srtFile;
      await loadFileInfo();
      if (!outputPath) {
        outputPath = generateOutputPath(inputPath, targetLang);
      }
    }
  }

  let providerOptions = $derived.by(() => {
    return [
      { id: "local", name: t("provider.local") },
      { id: "custom", name: t("provider.custom") },
      { id: "google", name: providers.google?.name || "Google Gemini" },
      { id: "groq", name: providers.groq?.name || "Groq" },
      { id: "openrouter", name: providers.openrouter?.name || "OpenRouter" },
      { id: "mistral", name: providers.mistral?.name || "Mistral AI" },
      { id: "github", name: providers.github?.name || "GitHub Models" },
      { id: "nvidia", name: providers.nvidia?.name || "NVIDIA NIM" },
    ];
  });

  // Custom providers saved in settings
  let savedCustomProviders = $derived(
    apiKeys.filter((k) => k.apiType === "custom" && k.apiUrl && k.apiUrl.trim().length > 0)
  );

  let availableModels = $derived.by(() => {
    if (!selectedProviderFamily) return [];
    // For local/custom providers, or when we successfully fetched dynamic models
    if (fetchedModels.length > 0) {
      const familyLabel = selectedProviderFamily === "local" || selectedProviderFamily === "custom" ? "Dynamic" : "Fetched API";
      return fetchedModels.map((m) => ({ id: m.id, name: m.name, provider: selectedProviderFamily, family: familyLabel }));
    }
    // Fallback to hardcoded models
    return getModelsForProvider(selectedProviderFamily);
  });

  // Effective model: use localCustomModel if filled (for local/custom provider), otherwise selectedModel
  let effectiveModel = $derived(
    (selectedProviderFamily === "local" || selectedProviderFamily === "custom") && localCustomModel.trim()
      ? localCustomModel.trim()
      : selectedModel
  );

  $effect(() => {
    const currentLang = targetLang;
    if (currentLang !== previousTargetLang) {
      if (inputPath && outputPath) {
        // Try to replace the previous lang code using separator-aware pattern
        const prevLangPattern = new RegExp(
          `([\\-._])${previousTargetLang.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\.srt$`,
          "i"
        );
        if (prevLangPattern.test(outputPath)) {
          outputPath = outputPath.replace(prevLangPattern, `$1${currentLang}.srt`);
        } else if (outputPath.endsWith(`.${previousTargetLang}.srt`)) {
          outputPath = outputPath.replace(
            new RegExp(`\\.${previousTargetLang}\\.srt$`, "i"),
            `.${currentLang}.srt`,
          );
        }
      } else if (inputPath && !outputPath) {
        outputPath = generateOutputPath(inputPath, currentLang);
      }
      previousTargetLang = currentLang;
    }
  });

  // Auto-select first model when provider changes (if current model invalid)
  $effect(() => {
    if (availableModels.length > 0) {
      const currentValid = availableModels.find((m) => m.id === selectedModel);
      if (!currentValid) {
        selectedModel = availableModels[0].id;
      }
    } else {
      selectedModel = "";
    }
  });

  $effect(() => {
    localStorage.setItem(LAST_PROVIDER_KEY, selectedProviderFamily || "");
    localStorage.setItem(LAST_MODEL_KEY, selectedModel || "");
    localStorage.setItem(LAST_TARGET_LANGUAGE_KEY, targetLang || "");
    localStorage.setItem(
      LAST_CUSTOM_PROVIDER_KEY,
      selectedCustomProviderId || "",
    );
    localStorage.setItem(LAST_CUSTOM_MODEL_KEY, localCustomModel || "");
  });



  // Fetch models dynamically for managed providers (Google, Groq)
  async function fetchProviderModels(family: string, force = false) {
    if (family !== "google" && family !== "groq") return;
    
    // Find the right API key
    const currentKeys = apiKeys.filter((k) => k.apiType === family && k.apiKey && k.apiKey.trim());
    if (currentKeys.length === 0) return;
    const apiKey = currentKeys[0].apiKey.trim();

    // Cache key based on family and a short hash/prefix of the API key to detect changes
    const cacheKey = `vesta-dynamic-models-${family}-${apiKey.substring(0, 8)}`;

    if (!force) {
      const cached = localStorage.getItem(cacheKey);
      if (cached) {
        try {
          const parsed = JSON.parse(cached);
          if (Array.isArray(parsed) && parsed.length > 0) {
            fetchedModels = parsed;
            isFetchingModels = false;
            fetchModelsError = "";
            if (!selectedModel || !parsed.find((m) => m.id === selectedModel)) {
              selectedModel = parsed[0].id;
            }
            return;
          }
        } catch (e) {}
      }
    }

    isFetchingModels = true;
    fetchModelsError = "";
    
    try {
      let url = "";
      let headers: Record<string, string> = { "Accept": "application/json" };
      
      if (family === "google") {
        url = `https://generativelanguage.googleapis.com/v1beta/models?key=${apiKey}`;
      } else if (family === "groq") {
        url = "https://api.groq.com/openai/v1/models";
        headers["Authorization"] = `Bearer ${apiKey}`;
      }

      const resp = await tauriFetch(url, { method: "GET", headers });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      
      const responseText = await resp.text();
      let data: unknown = null;
      if (responseText.trim().length > 0) {
         data = JSON.parse(responseText);
      }
      
      let models = extractModelsFromPayload(data);
      if (models.length === 0) throw new Error("No models found dynamically");

      // For Google, strip 'models/' prefix from the ID, use displayName if available,
      // and filter to only show models that support text generation (generateContent)
      if (family === "google") {
        if (data && typeof data === "object" && Array.isArray((data as any).models)) {
          models = (data as any).models
            .filter((m: any) => {
              // Only show models that support generateContent (text generation)
              const methods: string[] = m.supportedGenerationMethods || [];
              return methods.includes("generateContent");
            })
            .map((m: any) => {
               const id = m.name?.replace("models/", "") || m.id;
               const name = m.displayName || id;
               return { id, name };
            });
        } else {
             models = models.map(m => {
                 const id = m.id.replace("models/", "");
                 return { id, name: m.name.replace("models/", "") };
             });
        }
      }

      if (models.length === 0) throw new Error("No usable text generation models found");

      // For Google, probe each model with generateContent to check actual accessibility.
      if (family === "google") {
        const probeResults = await Promise.allSettled(
          models.map(async (m) => {
            const probeUrl = `https://generativelanguage.googleapis.com/v1beta/models/${m.id}:generateContent?key=${apiKey}`;
            const probeResp = await tauriFetch(probeUrl, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                contents: [{ parts: [{ text: "hi" }] }],
                generationConfig: { maxOutputTokens: 1 },
              }),
              signal: AbortSignal.timeout(8000),
            });
            if (!probeResp.ok) {
              throw new Error(`HTTP ${probeResp.status}`);
            }
            return m;
          })
        );
        const accessibleModels = probeResults
          .filter((r): r is PromiseFulfilledResult<{ id: string; name: string }> => r.status === "fulfilled")
          .map((r) => r.value);
        if (accessibleModels.length > 0) {
          models = accessibleModels;
        }
        // If all probes fail, keep original list as fallback
      }

      fetchedModels = models;
      localStorage.setItem(cacheKey, JSON.stringify(models));

      // Auto-select first model if current one is not in the list
      if (!selectedModel || !models.find((m) => m.id === selectedModel)) {
        selectedModel = models[0].id;
      }
    } catch (e: any) {
      console.warn(`Failed to fetch dynamic models for ${family}:`, e?.message);
      // We gracefully swallow errors and naturally fall back to hardcoded models (as fetchedModels remains [])
    } finally {
      isFetchingModels = false;
    }
  }

  // Auto-fetch provider models upon selection logic
  $effect(() => {
    if (providerConfirmed && (selectedProviderFamily === "google" || selectedProviderFamily === "groq")) {
      // Only trigger if we haven't fetched them yet to avoid loops
      if (fetchedModels.length === 0 && !isFetchingModels) {
        fetchProviderModels(selectedProviderFamily);
      }
    }
  });

  // Reset fetched models when switching away from the current provider
  $effect(() => {
    // Only reset if it's changing, avoid resetting immediately after we fetch
    if (!providerConfirmed) {
      fetchedModels = [];
      fetchModelsError = "";
    } else if (selectedProviderFamily !== "local" && selectedProviderFamily !== "custom" && selectedProviderFamily !== "google" && selectedProviderFamily !== "groq") {
      fetchedModels = [];
      fetchModelsError = "";
    }
  });

  let hasValidKey = $derived.by(() => {
    if (selectedProviderFamily === "local") return true;
    if (selectedProviderFamily === "google") {
      return apiKeys.some((k) => k.apiType === "google");
    }
    if (selectedProviderFamily === "groq") {
      return apiKeys.some((k) => k.apiType === "groq");
    }
    if (selectedProviderFamily === "custom") {
      if (!selectedCustomProviderId) return false;
      return apiKeys.some(
        (k) => k.id === selectedCustomProviderId && k.apiUrl && k.apiUrl.trim().length > 0,
      );
    }
    return false;
  });

  let hasCustomKey = $derived(apiKeys.some((k) => k.apiType === "custom" && k.apiUrl && k.apiUrl.trim().length > 0));
  let hasGoogleKey = $derived(apiKeys.some((k) => k.apiType === "google"));
  let hasGroqKey = $derived(apiKeys.some((k) => k.apiType === "groq"));
  let isLlmConfigured = $derived(
    !!selectedProviderFamily &&
      !!effectiveModel &&
      hasValidKey &&
      (selectedProviderFamily !== "local" || !!localServerUrl.trim()),
  );
  let translationBlockedReason = $derived(
    !useTiers
      ? "Configura almeno un Tier di traduzione nelle impostazioni (Settings > LLM & API Keys) prima di avviare la traduzione."
      : !inputPath || !outputPath
        ? "Seleziona file SRT di input e destinazione per abilitare la traduzione."
        : "",
  );

  let shouldKeepProviderPickerOpen = $derived(
    !providerConfirmed || (!hasValidKey && selectedProviderFamily !== "local"),
  );

  function handleStorageChange(e: StorageEvent) {
    if (e.key === "srt-tools-api-keys") {
      loadApiKeys();
    } else if (
      e.key === DEFAULT_LLM_PROVIDER_KEY ||
      e.key === DEFAULT_LLM_MODEL_KEY ||
      e.key === DEFAULT_LLM_CUSTOM_PROVIDER_KEY ||
      e.key === LOCAL_SERVER_URL_KEY
    ) {
      loadDefaultLlmSettings();
    } else if (e.key === DEFAULT_TARGET_LANGUAGE_KEY && !localStorage.getItem(LAST_TARGET_LANGUAGE_KEY)) {
      targetLang = loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY) || targetLang;
    }
  }

  function loadDefaultLlmSettings() {
    selectedProviderFamily = loadStoredValue(DEFAULT_LLM_PROVIDER_KEY) || selectedProviderFamily || "local";
    selectedModel = loadStoredValue(DEFAULT_LLM_MODEL_KEY) || selectedModel;
    selectedCustomProviderId = loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY) || selectedCustomProviderId;
    localServerUrl = localStorage.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL;
    providerConfirmed = true;
    fetchedModels = [];
    fetchModelsError = "";
  }

  $effect(() => {
    if (!active) {
      stopPreviewRefresh();
    } else if (isTranslating) {
      startPreviewRefresh();
    }
  });

  onMount(() => {
    loadApiKeys();
    refreshTiers();

    window.addEventListener("storage", handleStorageChange);

    // Also listen for custom event for same-window updates
    window.addEventListener("apikeys-updated", loadApiKeys);
    window.addEventListener("vesta-llm-default-updated", loadDefaultLlmSettings);
    window.addEventListener(TIERS_UPDATED_EVENT, refreshTiers);

    let activeListener = true;
    let unlistenDD: (() => void) | null = null;
    let unlistenProg: (() => void) | null = null;
    let unlistenComp: (() => void) | null = null;

    getCurrentWebview().onDragDropEvent((event) => {
      if (!active) return;
      if (event.payload.type === "over") isDraggingOver = true;
      else if (event.payload.type === "drop") {
        isDraggingOver = false;
        if (event.payload.paths) handleFileDrop(event.payload.paths);
      } else if (event.payload.type === "leave") isDraggingOver = false;
    }).then((fn) => {
      if (!activeListener) fn();
      else unlistenDD = fn;
    }).catch((e) => {
      console.warn("Failed to set up drag-drop listener:", e);
    });

    listen<TranslateProgressEvent>(
      "translate-progress",
      (event) => {
        progress = event.payload;
        addLog(event.payload.message);
      },
    ).then((fn) => {
      if (!activeListener) fn();
      else unlistenProg = fn;
    }).catch(console.error);

    listen<TranslateResult>(
      "translate-complete",
      (event) => {
        result = event.payload;
        isTranslating = false;
        stopPreviewRefresh();
        refreshTranslatedPreview();
        if (!event.payload.success || event.payload.translated_count > 0) {
          progress = null;
        }
        if (event.payload.output_path) {
          addLog(`📁 Saved: ${getFileName(event.payload.output_path)}`);
        }
        addLog(`✅ ${event.payload.message} (${event.payload.translated_count} subtitles)`);
      },
    ).then((fn) => {
      if (!activeListener) fn();
      else unlistenComp = fn;
    }).catch(console.error);

    return () => {
      activeListener = false;
      window.removeEventListener("storage", handleStorageChange);
      window.removeEventListener("apikeys-updated", loadApiKeys);
      window.removeEventListener("vesta-llm-default-updated", loadDefaultLlmSettings);
      window.removeEventListener(TIERS_UPDATED_EVENT, refreshTiers);
      if (unlistenDD) unlistenDD();
      if (unlistenProg) unlistenProg();
      if (unlistenComp) unlistenComp();
      stopPreviewRefresh();
    };
  });

  function loadApiKeys() {
    apiKeys = loadAndValidateApiKeys();
  }

  async function refreshTranslatedPreview() {
    if (!inputPath || !outputPath) return;

    try {
      const pairs = await invoke<SubtitlePair[]>(
        "get_latest_translated_subtitles",
        {
          inputPath: inputPath,
          outputPath: outputPath,
          count: 10, // Show last 10 translated subtitles
        },
      );
      translatedPairs = pairs;
    } catch (e) {
      // Silently ignore errors (file may not exist yet)
      console.debug("Preview refresh:", e);
    }
  }

  function startPreviewRefresh() {
    if (previewRefreshInterval) return;
    previewRefreshInterval = setInterval(refreshTranslatedPreview, 2000); // Refresh every 2 seconds
  }

  function stopPreviewRefresh() {
    if (previewRefreshInterval) {
      clearInterval(previewRefreshInterval);
      previewRefreshInterval = null;
    }
  }

  function addLog(message: string, type: LogEntry["type"] = "info") {
    const timestamp = new Date().toLocaleTimeString();
    logs = [...logs, { id: logIdCounter++, timestamp, message, type }];
  }

  async function selectInputFile() {
    if (!ensureProviderSelectedForFiles()) return;
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [{ name: "SRT Files", extensions: ["srt"] }],
      });

      if (selected) {
        inputPath = selected as string;
        await loadFileInfo();

        if (!outputPath) {
          outputPath = generateOutputPath(inputPath, targetLang);
        }
        addLog(`📥 Input selected: ${getFileName(inputPath)}`);
        addLog(`📤 Output set: ${getFileName(outputPath)}`);
      }
    } catch (e) {
      error = `${t("translate.errorSelectingFile")} ${e}`;
    }
  }

  async function selectOutputFile() {
    if (!ensureProviderSelectedForFiles()) return;
    try {
      const selected = await guardedSave({
        filters: [{ name: "SRT Files", extensions: ["srt"] }],
        defaultPath: outputPath || undefined,
      });

      if (selected) {
        outputPath = selected;
        addLog(`📤 Output updated: ${getFileName(outputPath)}`);
      }
    } catch (e) {
      error = `${t("translate.errorSelectingFile")} ${e}`;
    }
  }

  async function saveInputPath(newPath: string): Promise<boolean> {
    let cleaned = newPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      snackbar.show("Path cannot be empty", "error", 3500);
      return false;
    }
    try {
      const exists = await invoke<boolean>("transcribe_check_file_exists", {
        path: cleaned,
      });
      if (!exists) {
        snackbar.show(`File not found: ${cleaned}`, "error", 3500);
        return false;
      }
      inputPath = cleaned;
      await loadFileInfo();
      if (!outputPath) {
        outputPath = generateOutputPath(inputPath, targetLang);
      }
      addLog(`📥 Input selected: ${getFileName(inputPath)}`);
      addLog(`📤 Output set: ${getFileName(outputPath)}`);
      return true;
    } catch (e) {
      snackbar.show(`Error: ${e}`, "error", 3500);
      return false;
    }
  }

  async function saveOutputPath(newPath: string): Promise<boolean> {
    let cleaned = newPath.trim().replace(/\/+$/, "");
    if (!cleaned) {
      snackbar.show("Path cannot be empty", "error", 3500);
      return false;
    }
    const parentDir = cleaned.substring(0, cleaned.lastIndexOf("/"));
    if (parentDir) {
      try {
        const exists = await invoke<boolean>("transcribe_check_file_exists", {
          path: parentDir,
        });
        if (!exists) {
          snackbar.show(`Directory not found: ${parentDir}`, "error", 3500);
          return false;
        }
      } catch (e) {
        snackbar.show(`Error: ${e}`, "error", 3500);
        return false;
      }
    }
    outputPath = cleaned;
    addLog(`📤 Output updated: ${getFileName(outputPath)}`);
    return true;
  }

  async function loadFileInfo() {
    if (!inputPath) return;

    try {
      fileInfo = await invoke<SrtFileInfo>("load_srt_for_translate", {
        path: inputPath,
      });
      addLog(
        `📄 ${t("translate.loadedFile", { count: fileInfo.subtitle_count })}`,
      );
    } catch (e) {
      error = `${t("translate.errorLoading")} ${e}`;
      fileInfo = null;
    }
  }

  async function startTranslation() {
    if (!inputPath || !outputPath) {
      error = t("translate.selectFileAndKey");
      return;
    }

    const tiersPayload = buildTiersPayload();

    // La traduzione è guidata interamente dai tier: serve almeno un endpoint usabile.
    if (!tiersPayload) {
      error = t("tiers.noneConfigured");
      return;
    }

    error = null;
    result = null;
    progress = null;
    isTranslating = true;
    translatedPairs = [];
    addLog(`🚀 ${t("translate.starting")}`);
    addLog(`🌐 Target language: ${targetLang}`);
    addLog(`⚙️ Batch size: ${batchSize}, overlap: ${resumeOverlap}`);
    startPreviewRefresh();

    const endpointCount = tiersPayload.reduce((sum, tier) => sum + tier.length, 0);
    addLog(`🪜 ${t("tiers.logActive", { tiers: tiersPayload.length, endpoints: endpointCount })}`);

    const config: TranslateConfig = {
      input_path: inputPath,
      output_path: outputPath,
      target_lang: targetLang,
      batch_size: batchSize,
      resume_overlap: resumeOverlap,
      title_context: titleContext || null,
      tiers: tiersPayload,
    };

    try {
      const res = await invoke<TranslateResult>("start_translation", {
        config,
      });
      result = res;
      isTranslating = false;
    } catch (e: any) {
      let msg = e ? e.toString() : "Unknown error";
      const errorLower = msg.toLowerCase();

      let userMsg = msg;

      if (
        errorLower.includes("429") ||
        errorLower.includes("quota") ||
        errorLower.includes("rate limit")
      ) {
        userMsg = t("translate.error.ratelimit");
      } else if (
        errorLower.includes("401") ||
        errorLower.includes("unauthorized") ||
        errorLower.includes("api key")
      ) {
        userMsg = t("translate.error.apikey");
      } else if (errorLower.includes("json") || errorLower.includes("parse")) {
        userMsg = t("translate.error.json");
      }

      error = `${t("translate.errorTranslating")} ${userMsg}`;
      isTranslating = false;
      addLog(`❌ ${t("translate.error")}: ${msg}`);
    }
  }

  async function cancelTranslation() {
    try {
      await invoke("cancel_translation");
      isTranslating = false;
      progress = null;
      stopPreviewRefresh();
      addLog(`⚠️ ${t("translate.cancelled")}`);
    } catch (e) {
      error = `${t("translate.errorCancelling")} ${e}`;
    }
  }

  function resetTranslation() {
    result = null;
    error = null;
    progress = null;
    logs = [];
    translatedPairs = [];
    inputPath = "";
    outputPath = "";
    fileInfo = null;
  }

  function formatEta(seconds: number | null): string {
    if (seconds === null) return "...";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}m ${secs}s`;
  }

  function clearLogs() {
    logs = [];
  }

  function handleGoToSettings(section: "overview" | "llm" | "whisper" | "language" | "anki" = "llm") {
    if (onGoToSettings) {
      onGoToSettings(section);
    }
  }

  function showNoKeySnackbar(family: string) {
    snackbar.show("Configura prima un LLM nella macro-area Settings > LLM.", "warning", 4000);
  }

  const TRANSLATE_PANEL_IDS = [
    "options",
    "files",
    "progress",
    "livePreview",
    "logs",
  ] as const;

  type TranslatePanelId = (typeof TRANSLATE_PANEL_IDS)[number];
</script>

<div 
  role="region"
  aria-label="Translate content"
  class="h-full flex flex-col translate-tab-scroll relative overflow-hidden"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'; }}
  ondrop={(e) => { e.preventDefault(); isDraggingOver = false; }}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    isDraggingOver = false;
  }}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-green-500/10 border-2 border-dashed border-green-400 rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-green-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
          /></svg
        >
        <p class="text-lg font-medium text-green-300">
          {t("translate.dropFileHere")}
        </p>
        <p class="text-sm text-gray-400 mt-1">{t("translate.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  {#snippet panelContent(panelId: TranslatePanelId)}
    {#if panelId === "options"}
      <div class="glass-card p-5">
        <h3
          class="text-lg font-semibold mb-4 flex items-center gap-2 text-green-400"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
            />
          </svg>
          {t("translate.options")}
        </h3>
        <div class="space-y-4">


          <div
            class="{!inputPath
              ? 'opacity-40 pointer-events-none'
              : ''} transition-opacity space-y-4"
          >
            <div>
              <label for="target-lang" class="block text-sm text-gray-400 mb-1"
                >{t("translate.targetLang")}</label
              >
              <SearchableSelect
                noResultsText={t("common.noResults")}
                options={languages.map((lang) => ({
                  value: lang.code,
                  label:
                    lang.nameEn === lang.name
                      ? lang.name
                      : `${lang.nameEn} — ${lang.name}`,
                  searchTerms: getLanguageSearchTerms(lang.code),
                  icon: lang.flag,
                }))}
                value={targetLang}
                onchange={(v) => (targetLang = v)}
                placeholder={t("translate.targetLang")}
              />
            </div>
            <div>
              <div class="flex items-center justify-between mb-2">
                <span class="text-sm text-gray-400">{t("translate.batchSize")}</span>
              </div>
              <div class="grid grid-cols-4 gap-2">
                <button
                  onclick={() => setBatchPreset("precise")}
                  class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeBatchPreset ===
                  'precise'
                    ? 'bg-green-500/20 border-green-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                >
                  <span class="block mb-1 text-white">
                    <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <circle cx="12" cy="12" r="8" stroke-width="1.8" />
                      <circle cx="12" cy="12" r="2" stroke-width="1.8" />
                    </svg>
                  </span>
                  <span class="font-semibold block">{t("translate.batchPrecise")}</span>
                </button>
                <button
                  onclick={() => setBatchPreset("balanced")}
                  class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeBatchPreset ===
                  'balanced'
                    ? 'bg-green-500/20 border-green-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                >
                  <span class="block mb-1 text-white">
                    <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 4v16" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M6 8h12" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 8l-2.5 4h5L8 8z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M16 8l-2.5 4h5L16 8z" />
                    </svg>
                  </span>
                  <span class="font-semibold block">{t("translate.batchBalanced")}</span>
                </button>
                <button
                  onclick={() => setBatchPreset("fast")}
                  class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeBatchPreset ===
                  'fast'
                    ? 'bg-green-500/20 border-green-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                >
                  <span class="block mb-1 text-white">
                    <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 15l5-5 3 3 6-6" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M14 7h5v5" />
                    </svg>
                  </span>
                  <span class="font-semibold block">{t("translate.batchFast")}</span>
                </button>
                <button
                  onclick={() => setBatchPreset("turbo")}
                  class="p-2 rounded-lg text-center transition-all duration-200 border text-xs {activeBatchPreset ===
                  'turbo'
                    ? 'bg-green-500/20 border-green-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                >
                  <span class="block mb-1 text-white">
                    <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 3L6 13h5l-1 8 8-12h-5l2-6h-4z" />
                    </svg>
                  </span>
                  <span class="font-semibold block">{t("translate.batchTurbo")}</span>
                </button>
              </div>
              <div class="mt-2 flex items-center justify-end text-xs">
                <span
                  class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-sm"
                  >{batchSize} {t("translate.subPerBatch")}</span
                >
              </div>
            </div>
            <div class="mt-4">
              <div class="flex items-center justify-between mb-2">
                <label for="overlap-input" class="text-sm text-gray-400">
                  Resume Overlap Offset
                </label>
                <div class="flex items-center gap-2">
                  <span class="text-white font-mono bg-white/10 px-2 py-0.5 rounded text-xs">{resumeOverlap} sub</span>
                </div>
              </div>
              <input
                id="overlap-input"
                type="range"
                min="0"
                max="10"
                step="1"
                bind:value={resumeOverlap}
                class="w-full h-1.5 bg-gray-800 rounded-lg appearance-none cursor-pointer accent-green-500"
              />
            </div>
            
            <div class="mt-4">
              <label
                for="context-input"
                class="block text-sm text-gray-400 mb-1"
              >
                {t("translate.context")}
                <span class="text-gray-500"
                  >{t("translate.contextOptional")}</span
                >
              </label>
              <textarea
                id="context-input"
                bind:value={titleContext}
                rows="4"
                placeholder={t("translate.contextPlaceholder")}
                class="input-modern w-full text-sm min-h-[7rem] resize-y"
              ></textarea>
            </div>
          </div>
        </div>
      </div>
    {:else if panelId === "files"}
      <div
        inert={!canUseFilePanel}
        title={!canUseFilePanel ? "Select a provider first" : undefined}
        class="glass-card p-5 {!canUseFilePanel ? 'opacity-40' : ''}"
      >
        <h3
          class="text-lg font-semibold mb-4 flex items-center gap-2 panel-title-files-output"
        >
          <svg
            class="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
            />
          </svg>
          {t("common.filesAndOutput")}
        </h3>
        <div class="space-y-3">
          <PathPickerField
            label={t("translate.inputFile")}
            value={inputPath}
            placeholder={t("translate.selectFile")}
            browseTitle={t("translate.tooltip.upload")}
            onexpand={() => (expandedPathField = "input")}
            onbrowse={selectInputFile}
            required={true}
          />

          <PathPickerField
            label={t("translate.outputFile")}
            value={outputPath}
            placeholder={t("translate.selectDestination")}
            browseTitle={t("translate.tooltip.save")}
            onexpand={() => (expandedPathField = "output")}
            onbrowse={selectOutputFile}
            required={true}
          />
          {#if fileInfo}
            <div
              class="p-3 bg-indigo-500/10 border border-indigo-500/30 rounded-lg"
            >
              <div class="flex items-center gap-3">
                <div
                  class="w-10 h-10 rounded-lg bg-indigo-500/20 flex items-center justify-center"
                >
                  <span class="text-xl">📄</span>
                </div>
                <div>
                  <p class="font-medium text-white">
                    {fileInfo.subtitle_count}
                    {t("translate.subtitles")}
                  </p>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>

    {:else if panelId === "progress"}
      <div class="space-y-3">
        {#if isTranslating || progress}
          <div
            class="glass-card p-4 animate-fade-in shrink-0 {isTranslating
              ? 'animate-pulse-glow'
              : ''}"
          >
            <div class="flex items-center gap-6">
              <div class="flex-1">
                <div class="progress-modern h-2">
                  <div
                    class="progress-modern-bar"
                    style="width: {progress?.percentage || 0}%"
                  ></div>
                </div>
              </div>
              <span class="text-gray-400 text-sm whitespace-nowrap">
                {t("translate.batch")}
                {progress?.current_batch || 0}/{progress?.total_batches || 0}
              </span>
              <span
                class="text-xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent"
              >
                {Math.round(progress?.percentage || 0)}%
              </span>
              {#if progress?.eta_seconds}
                <span class="text-gray-500 text-sm flex items-center gap-1">
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                    />
                  </svg>
                  {formatEta(progress.eta_seconds)}
                </span>
              {/if}
            </div>
          </div>
        {/if}
        {#if result}
          <div
            class="glass-card p-4 border-l-4 animate-fade-in shrink-0 {result.success
              ? 'border-green-500 bg-green-500/5'
              : 'border-red-500 bg-red-500/5'}"
          >
            <div class="flex items-center gap-3">
              {#if result.success}
                <svg
                  class="w-5 h-5 text-green-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              {:else}
                <svg
                  class="w-5 h-5 text-red-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              {/if}
              <div class="flex-1">
                <p
                  class="{result.success
                    ? 'text-green-400'
                    : 'text-red-400'} font-medium"
                >
                  {result.message}
                </p>
                {#if result.output_path}
                  <p class="text-xs text-gray-500 mt-1 font-mono truncate">
                    📁 {result.output_path}
                  </p>
                {/if}
              </div>
            </div>
          </div>
        {/if}
        {#if error}
          <div
            class="glass-card p-4 border border-red-500/30 bg-red-500/10 animate-fade-in shrink-0"
          >
            <div class="flex items-center gap-3">
              <svg
                class="w-5 h-5 text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <p class="text-red-300 flex-1 text-sm">{error}</p>
              <button
                onclick={() => (error = null)}
                class="text-red-400 hover:text-red-300">✕</button
              >
            </div>
          </div>
        {/if}
      </div>
    {:else if panelId === "livePreview"}
      <div class="glass-card p-5 shrink-0 min-h-[400px]">
        <div class="flex items-center justify-between mb-4">
          <h3
            class="text-lg font-semibold flex items-center gap-2 text-purple-400"
          >
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            {t("translate.livePreview")}
            {#if translatedPairs.length > 0}
              <span class="text-xs text-gray-500 font-normal ml-2"
                >({translatedPairs.length} {t("translate.subtitles")})</span
              >
            {/if}
          </h3>
        </div>
        <div class="grid grid-cols-2 gap-4 min-h-[340px]">
          <div class="bg-white/5 rounded-xl p-4 flex flex-col">
            <p
              class="text-xs text-gray-500 uppercase tracking-wide mb-3 shrink-0"
            >
              {t("translate.original")}
            </p>
            <div class="flex-1 overflow-y-auto space-y-2">
              {#if translatedPairs.length > 0}
                {#each translatedPairs as pair}
                  <div
                    class="p-2 bg-black/20 rounded-lg border-l-2 border-gray-600"
                  >
                    <span class="text-[10px] text-gray-600 font-mono"
                      >#{pair.id}</span
                    >
                    <p class="text-gray-300 text-sm mt-0.5">{pair.original}</p>
                  </div>
                {/each}
              {:else}
                <div class="flex items-center justify-center h-full">
                  <p class="text-gray-600 text-sm">
                    {t("translate.waitingForTranslation")}
                  </p>
                </div>
              {/if}
            </div>
          </div>
          <div class="bg-white/5 rounded-xl p-4 flex flex-col">
            <p
              class="text-xs text-gray-500 uppercase tracking-wide mb-3 shrink-0"
            >
              {t("translate.translated")}
            </p>
            <div class="flex-1 overflow-y-auto space-y-2">
              {#if translatedPairs.length > 0}
                {#each translatedPairs as pair}
                  <div
                    class="p-2 bg-green-500/5 rounded-lg border-l-2 border-green-500/50"
                  >
                    <span class="text-[10px] text-green-600/70 font-mono"
                      >#{pair.id}</span
                    >
                    <p class="text-green-300 text-sm mt-0.5">
                      {pair.translated}
                    </p>
                  </div>
                {/each}
              {:else}
                <div class="flex items-center justify-center h-full">
                  <p class="text-gray-600 text-sm">
                    {t("translate.waitingForTranslation")}
                  </p>
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {:else if panelId === "logs"}
      <LogPanel
        title={t("translate.logs")}
        clearLogText={t("translate.clearLog")}
        noLogText={t("translate.noLog")}
        {logs}
        onclear={clearLogs}
        minHeight="190px"
        maxHeightContent="16rem"
      />
    {/if}
  {/snippet}

  <div class="flex-1 overflow-y-auto p-6 min-h-0">
    <div class="grid grid-cols-1 xl:grid-cols-2 gap-6 transition-opacity">
      <div class="space-y-3 overflow-x-hidden min-h-[100px]">
        {@render panelContent("files")}
        {@render panelContent("progress")}
        {@render panelContent("livePreview")}
      </div>

      <div class="space-y-3 overflow-x-hidden min-h-[100px]">
        {@render panelContent("options")}
        <!-- {@render panelContent("logs")} -->
      </div>
    </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <div class="h-[92px] border-t border-white/10 bg-gray-900 flex items-center justify-end gap-4 px-6 shrink-0 z-40">
    {#if isTranslating}
      <button
        onclick={cancelTranslation}
        class="px-5 py-2.5 bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-red-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-red-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer"
      >
        <svg
          class="w-4 h-4 text-red-100"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
        {t("translate.cancel")}
      </button>
    {:else if result || error}
      <div class="relative group">
        <button
          onclick={resetTranslation}
          class="px-5 py-2.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-300 rounded-xl font-bold text-sm transition-all border border-amber-500/30 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
        >
          <svg
            class="w-4 h-4 text-amber-300"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
          New Translation
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
          {t("translate.cancel") === "Annulla" ? "Azzera e avvia una nuova traduzione" : "Reset and start a new translation"}
        </div>
      </div>
    {:else}
      <div class="relative group">
        <button
          onclick={startTranslation}
          disabled={!inputPath || !outputPath || !useTiers}
          class="px-5 py-2.5 bg-emerald-600/80 hover:bg-emerald-500/80 border border-emerald-500/30 disabled:bg-emerald-600/40 text-emerald-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-55 cursor-pointer"
        >
          <svg
            class="w-4 h-4 text-emerald-100"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          {t("translate.start")}
        </button>
        <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {translationBlockedReason || t("translate.start")}
        </div>
      </div>
    {/if}
  </div>

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input"
      ? t("translate.inputFile")
      : t("translate.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
    editable={true}
    secondaryText={`✏️ ${t("transcribe.editPath")}`}
    desc={expandedPathField === "input"
      ? t("transcribe.inputPathDesc")
      : t("transcribe.outputPathDesc")}
    onsave={expandedPathField === "input" ? saveInputPath : saveOutputPath}
  />
</div>

<style>
  .translate-tab-scroll {
    background: #111827;
    contain: layout style;
    -webkit-overflow-scrolling: touch;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
</style>

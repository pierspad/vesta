<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { fetch as tauriFetch } from "./tauriHttp";
  import { onDestroy, onMount } from "svelte";
  import { locale, currentLanguage } from "./i18n";
  import { getFileName } from "./models";
  import { loadAndValidateApiKeys, type ApiKeyConfig } from "./apiKeys";
  import {
    loadTiers,
    tiersHaveUsableEntries,
    TIERS_UPDATED_EVENT,
    type Tier,
  } from "./translationTiers";
  import { getModelsForProvider, providers } from "./llmProviders";
  import { detectLanguageCode, getLanguageSearchTerms, languages } from "./languages";
  import PathPickerField from "./PathPickerField.svelte";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import { snackbar } from "./snackbarStore.svelte";
  import { aiStore } from "./aiStore.svelte";
  import FooterActions from "./components/FooterActions.svelte";
  import { uiMode } from "./uiModeStore.svelte";
  import {
    extractModelsFromPayload,
    fetchModelsFromEndpoint,
    type DiscoveredModel,
  } from "./modelDiscovery";
  import {
    buildTiersPayload as buildTiersPayloadShared,
    checkTiersAvailability as checkTiersAvailabilityShared,
    type TierEntryPayload,
    type TiersUnavailableReason,
  } from "./llmTiers";
  import * as vestaConfig from "./vestaConfig";
  import {
    getLatestTranslatedSubtitles,
    loadSrtForTranslate,
    startTranslation as apiStartTranslation,
    cancelTranslation as apiCancelTranslation,
    type SrtFileInfo,
    type TranslateConfig,
    type TranslateResult,
    type SubtitlePair,
  } from "./services/translate";
  import { transcribeCheckFileExists } from "./services/transcribe";

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

  interface TranslateProgressEvent {
    message: string;
    current_batch: number;
    total_batches: number;
    percentage: number;
    eta_seconds: number | null;
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
      return vestaConfig.getItem(key) || "";
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
  let localServerUrl = $state(vestaConfig.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL);

  // Dynamically fetched models from local/custom server
  let fetchedModels = $state<DiscoveredModel[]>([]);
  let isFetchingModels = $state(false);
  let fetchModelsError = $state("");

  function saveLocalServerUrl(url: string) {
    localServerUrl = url;
    vestaConfig.setItem(LOCAL_SERVER_URL_KEY, url);
  }

  async function fetchModelsFromServer(baseUrl: string, force = false) {
    // Only cache if it's for the selected custom provider, to avoid mixing URLs.
    // For local, we don't cache since it runs locally and is fast, but we could. Let's just cache custom.
    const cacheKey = selectedProviderFamily === "custom" && selectedCustomProviderId
        ? `vesta-dynamic-models-custom-${selectedCustomProviderId}`
        : null;

    if (!force && cacheKey) {
      const cached = vestaConfig.getItem(cacheKey);
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
        vestaConfig.setItem(cacheKey, JSON.stringify(models));
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
    { id: "fast", value: 50 },
    { id: "turbo", value: 100 },
  ] as const;
  let activeBatchPreset = $derived(
    (() => {
      let closest: typeof batchPresets[number] = batchPresets[0];
      let minDiff = Math.abs(batchSize - closest.value);
      for (const preset of batchPresets) {
        const diff = Math.abs(batchSize - preset.value);
        if (diff < minDiff) {
          minDiff = diff;
          closest = preset;
        }
      }
      return closest.id;
    })()
  );
  function setBatchPreset(presetId: string) {
    const preset = batchPresets.find((p) => p.id === presetId);
    if (preset) batchSize = preset.value;
  }

  const overlapPresets = [
    { id: "none", value: 0 },
    { id: "minimal", value: 1 },
    { id: "balanced", value: 2 },
    { id: "high", value: 5 },
  ] as const;
  let activeOverlapPreset = $derived(
    (() => {
      let closest: typeof overlapPresets[number] = overlapPresets[0];
      let minDiff = Math.abs(resumeOverlap - closest.value);
      for (const preset of overlapPresets) {
        const diff = Math.abs(resumeOverlap - preset.value);
        if (diff < minDiff) {
          minDiff = diff;
          closest = preset;
        }
      }
      return closest.id;
    })()
  );
  function setOverlapPreset(presetId: string) {
    const preset = overlapPresets.find((p) => p.id === presetId);
    if (preset) resumeOverlap = preset.value;
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

  // Payload a tier: delega al modulo condiviso (stessa logica di RefineTab).
  function buildTiersPayload(): TierEntryPayload[][] | null {
    if (!useTiers) return null;
    return buildTiersPayloadShared(tiers, apiKeys);
  }

  function tiersUnavailableMessage(reason: TiersUnavailableReason): string {
    switch (reason) {
      case "noneConfigured":
        return t("tiers.noneConfigured") || "No tiers configured";
      case "localOffline":
        return t("settings.llmConfigIncompleteDescLocalOffline") || "The local LLM server is offline. Please start Ollama/LM Studio or verify the endpoint URL.";
      case "keyMissing":
        return t("settings.llmConfigIncompleteDescKey") || "Missing API key";
      case "incomplete":
        return t("settings.llmConfigIncompleteDescCustomEmpty") || "LLM configuration incomplete";
      default:
        return t("translate.errorTranslating") || "No available LLM found in tiers. Check your settings.";
    }
  }

  async function checkTiersAvailability(): Promise<{ available: boolean; errorMsg?: string }> {
    if (!useTiers) {
      return { available: false, errorMsg: t("tiers.noneConfigured") || "No tiers configured" };
    }
    const check = await checkTiersAvailabilityShared(tiers, apiKeys);
    if (check.available) return { available: true };
    return { available: false, errorMsg: tiersUnavailableMessage(check.reason) };
  }

  let unlistenProgress: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  let unlistenDragDrop: (() => void) | null = null;
  let isDraggingOver = $state(false);

  let canUseFilePanel = $derived(providerConfirmed);

  function ensureProviderSelectedForFiles(): boolean {
    if (providerConfirmed) return true;
    error = t("translate.selectProviderFirst");
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
    vestaConfig.setItem(LAST_PROVIDER_KEY, selectedProviderFamily || "");
    vestaConfig.setItem(LAST_MODEL_KEY, selectedModel || "");
    vestaConfig.setItem(LAST_TARGET_LANGUAGE_KEY, targetLang || "");
    vestaConfig.setItem(
      LAST_CUSTOM_PROVIDER_KEY,
      selectedCustomProviderId || "",
    );
    vestaConfig.setItem(LAST_CUSTOM_MODEL_KEY, localCustomModel || "");
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
      const cached = vestaConfig.getItem(cacheKey);
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
              timeoutMs: 8000,
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
      vestaConfig.setItem(cacheKey, JSON.stringify(models));

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
      ? t("translate.blocked.noTiers")
      : !inputPath || !outputPath
        ? t("translate.blocked.selectFiles")
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
    } else if (e.key === DEFAULT_TARGET_LANGUAGE_KEY && !vestaConfig.getItem(LAST_TARGET_LANGUAGE_KEY)) {
      targetLang = loadStoredValue(DEFAULT_TARGET_LANGUAGE_KEY) || targetLang;
    }
  }

  function loadDefaultLlmSettings() {
    selectedProviderFamily = loadStoredValue(DEFAULT_LLM_PROVIDER_KEY) || selectedProviderFamily || "local";
    selectedModel = loadStoredValue(DEFAULT_LLM_MODEL_KEY) || selectedModel;
    selectedCustomProviderId = loadStoredValue(DEFAULT_LLM_CUSTOM_PROVIDER_KEY) || selectedCustomProviderId;
    localServerUrl = vestaConfig.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL;
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
      const pairs = await getLatestTranslatedSubtitles(
        inputPath,
        outputPath,
        10, // Show last 10 translated subtitles
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
      const exists = await transcribeCheckFileExists(cleaned);
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
        const exists = await transcribeCheckFileExists(parentDir);
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
      fileInfo = await loadSrtForTranslate(inputPath);
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

    const check = await checkTiersAvailability();
    if (!check.available) {
      if (check.errorMsg) {
        snackbar.show(check.errorMsg, "error");
      }
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
      const res = await apiStartTranslation(config);
      result = res;
      isTranslating = false;
    } catch (e: any) {
      let msg = e ? e.toString() : "Unknown error";
      const errorLower = msg.toLowerCase();

      let userMsg = msg;

      if (msg.includes("ERR_ALREADY_RUNNING")) {
        userMsg = t("common.error.alreadyRunning");
      } else if (
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
      await apiCancelTranslation();
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

  /** Localized, count-aware message for the result box (backend text is a fallback). */
  function resultMessage(r: TranslateResult): string {
    const raw = (r.message || "").toLowerCase();
    if (raw.includes("annullat") || raw.includes("cancel")) return t("translate.cancelled");
    if (r.success) return t("translate.resultDone", { count: String(r.translated_count) });
    if (r.translated_count === 0) return t("translate.resultNone");
    return r.message;
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

  function showNoKeySnackbar(_family: string) {
    snackbar.show(t("translate.configureLlmFirst"), "warning", 4000);
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
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden"
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
            {#if uiMode.expertMode}
              <!-- Expert/Advanced Translation Options -->
              <div>
                <div class="flex items-center justify-between mb-3">
                  <span class="text-sm font-semibold text-white">{t("translate.batchSizeExpert")}</span>
                  <span class="text-white font-mono bg-white/10 px-2.5 py-1 rounded-lg text-xs shrink-0">
                    {batchSize} {t("translate.subPerBatch")}
                  </span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="100"
                  bind:value={batchSize}
                  class="slider-resource w-full cursor-pointer"
                />
                <!-- Tick marks for batch size -->
                <div class="relative mt-1.5 mb-5" style="height: 22px;">
                  {#each [1, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100] as v}
                    {@const pct = ((v - 1) / 99) * 100}
                    <div
                      class="absolute flex flex-col items-center gap-0.5"
                      style="left: {pct}%; transform: translateX(-50%);"
                    >
                      <div class="w-px h-1.5 {batchSize === v ? 'bg-white/60' : 'bg-white/20'}"></div>
                      <span class="text-[9px] {batchSize === v ? 'text-white/70' : 'text-white/25'}">{v}</span>
                    </div>
                  {/each}
                </div>

                <div class="pt-4 border-t border-white/5 mb-4">
                  <div class="flex items-center justify-between mb-3">
                    <span class="text-sm font-semibold text-white">{t("translate.resumeOverlapExpert")}</span>
                    <span class="text-white font-mono bg-white/10 px-2.5 py-1 rounded-lg text-xs shrink-0">
                      {resumeOverlap} sub
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="15"
                    bind:value={resumeOverlap}
                    class="slider-resource w-full cursor-pointer"
                  />
                  <!-- Tick marks for overlap -->
                  <div class="relative mt-1.5" style="height: 22px;">
                    {#each Array(16) as _, i}
                      {@const pct = (i / 15) * 100}
                      <div
                        class="absolute flex flex-col items-center gap-0.5"
                        style="left: {pct}%; transform: translateX(-50%);"
                      >
                        <div class="w-px h-1.5 {resumeOverlap === i ? 'bg-white/60' : 'bg-white/20'}"></div>
                        {#if i === 0 || i === 5 || i === 10 || i === 15}
                          <span class="text-[9px] {resumeOverlap === i ? 'text-white/70' : 'text-white/25'}">{i}</span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
                
                <div class="pt-4 border-t border-white/5">
                  <label
                    for="context-input"
                    class="block text-sm font-semibold text-white mb-1"
                  >
                    {t("translate.context")}
                    <span class="text-gray-500 font-normal">({t("translate.contextOptional")})</span>
                  </label>
                  <textarea
                    id="context-input"
                    bind:value={titleContext}
                    rows="3"
                    placeholder={t("translate.contextPlaceholder")}
                    class="input-modern w-full text-xs min-h-[5rem] resize-y"
                  ></textarea>
                </div>
              </div>
            {:else}
              <!-- Easy/Simplified Translation Options -->
              <div>
                <!-- Accuracy and speed preset block -->
                <div class="mb-5">
                  <span class="block text-sm font-semibold text-white mb-3">
                    {t("translate.accuracySpeedTitle")}
                  </span>
                  <div class="grid grid-cols-4 gap-2">
                    <button
                      type="button"
                      onclick={() => setBatchPreset("precise")}
                      class="p-2 rounded-lg text-center transition-all duration-200 border text-xs cursor-pointer
                        {activeBatchPreset === 'precise'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                    >
                      <span class="block mb-1 text-white">
                        <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <circle cx="12" cy="12" r="8" stroke-width="1.8" />
                          <circle cx="12" cy="12" r="2" stroke-width="1.8" />
                        </svg>
                      </span>
                      <span class="block">{t("translate.batchPrecise")}</span>
                    </button>
                    <button
                      type="button"
                      onclick={() => setBatchPreset("balanced")}
                      class="p-2 rounded-lg text-center transition-all duration-200 border text-xs cursor-pointer
                        {activeBatchPreset === 'balanced'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
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
                      <span class="block">{t("translate.batchBalanced")}</span>
                    </button>
                    <button
                      type="button"
                      onclick={() => setBatchPreset("fast")}
                      class="p-2 rounded-lg text-center transition-all duration-200 border text-xs cursor-pointer
                        {activeBatchPreset === 'fast'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                    >
                      <span class="block mb-1 text-white">
                        <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M5 15l5-5 3 3 6-6" />
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M14 7h5v5" />
                        </svg>
                      </span>
                      <span class="block">{t("translate.batchFast")}</span>
                    </button>
                    <button
                      type="button"
                      onclick={() => setBatchPreset("turbo")}
                      class="p-2 rounded-lg text-center transition-all duration-200 border text-xs cursor-pointer
                        {activeBatchPreset === 'turbo'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                    >
                      <span class="block mb-1 text-white">
                        <svg class="w-4 h-4 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 3L6 13h5l-1 8 8-12h-5l2-6h-4z" />
                        </svg>
                      </span>
                      <span class="block">{t("translate.batchTurbo")}</span>
                    </button>
                  </div>
                </div>

                <!-- Text continuity preset block -->
                <div class="pt-4 border-t border-white/5">
                  <div class="flex items-center gap-2 mb-3">
                    <span class="block text-sm font-semibold text-white">
                      {t("translate.resumeOverlap")}
                    </span>
                    <span class="text-[9px] uppercase tracking-wide font-semibold px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-400 border border-amber-500/25">
                      {t("translate.contextCostHint")}
                    </span>
                  </div>
                  <div class="grid grid-cols-4 gap-2">
                    <button
                      type="button"
                      onclick={() => setOverlapPreset('none')}
                      class="p-2.5 rounded-lg text-center transition-all duration-200 border text-[11px] cursor-pointer truncate
                        {activeOverlapPreset === 'none'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                      title={t("translate.overlapNone")}
                    >
                      {t("translate.overlapNone")}
                    </button>
                    <button
                      type="button"
                      onclick={() => setOverlapPreset('minimal')}
                      class="p-2.5 rounded-lg text-center transition-all duration-200 border text-[11px] cursor-pointer truncate
                        {activeOverlapPreset === 'minimal'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                      title={t("translate.overlapMinimal")}
                    >
                      {t("translate.overlapMinimal")}
                    </button>
                    <button
                      type="button"
                      onclick={() => setOverlapPreset('balanced')}
                      class="p-2.5 rounded-lg text-center transition-all duration-200 border text-[11px] cursor-pointer truncate
                        {activeOverlapPreset === 'balanced'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                      title={t("translate.overlapNormal")}
                    >
                      {t("translate.overlapNormal")}
                    </button>
                    <button
                      type="button"
                      onclick={() => setOverlapPreset('high')}
                      class="p-2.5 rounded-lg text-center transition-all duration-200 border text-[11px] cursor-pointer truncate
                        {activeOverlapPreset === 'high'
                          ? 'bg-green-500/20 border-green-500/50 text-white font-semibold shadow-sm'
                          : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                      title={t("translate.overlapHigh")}
                    >
                      {t("translate.overlapHigh")}
                    </button>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {:else if panelId === "files"}
      <div
        inert={!canUseFilePanel}
        title={!canUseFilePanel ? t("translate.selectProviderFirst") : undefined}
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
                  {resultMessage(result)}
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
        {@render panelContent("livePreview")}
      </div>

      <div class="space-y-3 overflow-x-hidden min-h-[100px]">
        {@render panelContent("options")}
        <!-- {@render panelContent("logs")} -->
      </div>
    </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions>
    {#snippet background()}
      {#if isTranslating}
        <div
          class="absolute inset-y-0 left-0 bg-gradient-to-r from-purple-500/15 to-pink-500/20 transition-all duration-300 ease-out z-0 pointer-events-none"
          style="width: {progress?.percentage || 0}%"
        ></div>
        <div class="absolute inset-0 bg-shimmer-stripes opacity-15 z-0 pointer-events-none"></div>
      {/if}
    {/snippet}
    {#snippet left()}
    <!-- Left side: Progress/Result status info -->
    <div class="flex items-center gap-4 select-none z-10 min-w-0 flex-1">
      {#if isTranslating}
        <!-- Loading spinner and status message -->
        <div class="flex items-center gap-3">
          <div class="w-5 h-5 border-2 border-purple-400 border-t-transparent rounded-full animate-spin"></div>
          <div class="flex flex-col">
            <span class="text-[10px] text-purple-400 font-bold uppercase tracking-wider">{t("translate.translating") || ($currentLanguage === 'it' ? 'Traduzione in corso...' : 'Translating...')}</span>
            <span class="text-xs text-white font-medium truncate max-w-lg">
              {t("translate.batch")} {progress?.current_batch || 0}/{progress?.total_batches || 0} ({Math.round(progress?.percentage || 0)}%)
              {#if progress?.eta_seconds}
                <span class="text-gray-400 ml-2">{t("translate.eta", { time: formatEta(progress.eta_seconds) })}</span>
              {/if}
            </span>
          </div>
        </div>
      {:else if result}
        <!-- Success/Result state -->
        <div class="flex items-center gap-3">
          {#if result.success}
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-emerald-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-emerald-400 font-bold uppercase tracking-wider">{t("translate.finished") || ($currentLanguage === 'it' ? 'Completato' : 'Completed')}</span>
              {#if result.output_path}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div 
                  onclick={() => {
                    if (result?.output_path) {
                      navigator.clipboard.writeText(result.output_path);
                      snackbar.show($currentLanguage === 'it' ? 'Percorso copiato negli appunti!' : 'Path copied to clipboard!', 'success');
                    }
                  }}
                  class="flex items-center gap-1.5 text-xs text-gray-400 mt-0.5 cursor-pointer hover:text-white transition-colors bg-white/5 hover:bg-white/10 px-2 py-0.5 rounded border border-white/5 select-all"
                  title={$currentLanguage === 'it' ? 'Clicca per copiare il percorso' : 'Click to copy path'}
                >
                  <span class="truncate max-w-sm">📁 {getFileName(result.output_path)}</span>
                  <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
                  </svg>
                </div>
              {:else}
                <span class="text-xs text-white font-medium">{result.message}</span>
              {/if}
            </div>
          {:else}
            <!-- Failure state -->
            <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </div>
            <div class="flex flex-col">
              <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("translate.error") || ($currentLanguage === 'it' ? 'Errore' : 'Error')}</span>
              <span class="text-xs text-red-300 font-medium truncate max-w-lg">{result.message}</span>
            </div>
          {/if}
        </div>
      {:else if error}
        <!-- Error State -->
        <div class="flex items-center gap-3">
          <div class="flex items-center justify-center w-8 h-8 rounded-full bg-red-500/10 border border-red-500/30 text-red-400">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div class="flex flex-col">
            <span class="text-[10px] text-red-400 font-bold uppercase tracking-wider">{t("translate.error") || ($currentLanguage === 'it' ? 'Errore' : 'Error')}</span>
            <span class="text-xs text-red-300 font-medium truncate max-w-lg">{error}</span>
          </div>
        </div>
      {/if}
    </div>
    {/snippet}
    {#snippet right()}
    <!-- Right side: Action Buttons -->
    <div class="flex items-center gap-4 z-10 select-none shrink-0">
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
            onclick={startTranslation}
            class="px-5 py-2.5 bg-emerald-600/80 hover:bg-emerald-500/80 border border-emerald-500/30 text-emerald-100 rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 hover:scale-[1.02] active:scale-[0.98] cursor-pointer"
          >
            <svg class="w-4 h-4 text-emerald-100" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            {t("translate.retry")}
          </button>
          <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-emerald-500/30 bg-gray-950/95 p-3 text-center text-xs text-emerald-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
            {t("translate.retryDesc")}
          </div>
        </div>
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
            {t("translate.newTranslation")}
          </button>
          <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap">
            {t("translate.newTranslationDesc")}
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
    {/snippet}
  </FooterActions>

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
  @keyframes progress-stripes {
    0% { background-position: 0 0; }
    100% { background-position: 40px 0; }
  }
  .bg-shimmer-stripes {
    background-image: linear-gradient(
      45deg,
      rgba(139, 92, 246, 0.15) 25%,
      transparent 25%,
      transparent 50%,
      rgba(139, 92, 246, 0.15) 50%,
      rgba(139, 92, 246, 0.15) 75%,
      transparent 75%,
      transparent
    );
    background-size: 40px 40px;
    animation: progress-stripes 1.2s linear infinite;
  }
</style>

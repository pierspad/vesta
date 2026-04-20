<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "./dialogGuard";
  import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
  import { onDestroy, onMount } from "svelte";
  import { locale } from "./i18n";
  import {
    getModelsForProvider,
    languages,
    loadAndValidateApiKeys,
    type ApiKeyConfig,
  } from "./models";
  import PathPickerField from "./PathPickerField.svelte";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import SearchableSelect from "./SearchableSelect.svelte";
  import LogPanel, { type LogEntry } from "./LogPanel.svelte";
  import InfoModal from "./InfoModal.svelte";
  import InfoButton from "./InfoButton.svelte";
  import { translateSections } from "./info";

  // Set of known language codes for smart output filename detection
  const knownLangCodes = new Set(languages.map((l) => l.code));
  // Also add common 3-letter ISO 639-2 codes
  const extraLangCodes = [
    "eng", "ita", "spa", "fra", "deu", "por", "rus", "jpn", "kor", "zho",
    "ara", "tur", "pol", "nld", "swe", "nor", "dan", "fin", "ces", "hun",
    "ron", "ukr", "ell", "heb", "hin", "ind", "msa", "tha", "vie", "isl", "cat",
  ];
  for (const c of extraLangCodes) knownLangCodes.add(c);

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
      if (knownLangCodes.has(lastSegment.toLowerCase())) {
        return `${prefix}${separator}${lang}.srt`;
      }
    }
    // Fallback: append .lang.srt
    return input.replace(/\.srt$/i, `.${lang}.srt`);
  }

  interface Props {
    onGoToSettings?: () => void;
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

  interface TranslateConfig {
    input_path: string;
    output_path: string;
    target_lang: string;
    api_keys: string[];
    api_type: string;
    batch_size: number;
    resume_overlap: number;
    title_context: string | null;
    api_url: string | null;
    model: string | null;
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

  function loadStoredValue(key: string): string {
    try {
      return localStorage.getItem(key) || "";
    } catch {
      return "";
    }
  }

  let inputPath = $state("");
  let outputPath = $state("");
  let targetLang = $state("it");
  let previousTargetLang = "it";
  const initialProvider = loadStoredValue(LAST_PROVIDER_KEY);
  let selectedProviderFamily = $state(initialProvider);
  let providerConfirmed = $state(Boolean(initialProvider));
  let selectedCustomProviderId = $state(loadStoredValue(LAST_CUSTOM_PROVIDER_KEY));
  let tempSnackbar = $state("");
  let tempSnackbarTimer: ReturnType<typeof setTimeout> | null = null;
  let localCustomModel = $state(loadStoredValue(LAST_CUSTOM_MODEL_KEY));
  let batchSize = $state(15);
  let resumeOverlap = $state(2);
  let titleContext = $state("");
  let selectedModel = $state(loadStoredValue(LAST_MODEL_KEY));

  // Local server URL with persistence
  const LOCAL_SERVER_URL_KEY = "vesta-local-server-url";
  const DEFAULT_LOCAL_URL = "http://localhost:11434/v1";
  let localServerUrl = $state(localStorage.getItem(LOCAL_SERVER_URL_KEY) || DEFAULT_LOCAL_URL);

  // Dynamically fetched models from local/custom server
  let fetchedModels = $state<{ id: string; name: string }[]>([]);
  let isFetchingModels = $state(false);
  let fetchModelsError = $state("");

  function saveLocalServerUrl(url: string) {
    localServerUrl = url;
    localStorage.setItem(LOCAL_SERVER_URL_KEY, url);
  }

  function extractModelsFromPayload(payload: unknown): { id: string; name: string }[] {
    const candidates: unknown[] = [];

    if (Array.isArray(payload)) {
      candidates.push(...payload);
    } else if (payload && typeof payload === "object") {
      const record = payload as Record<string, unknown>;

      if (Array.isArray(record.data)) {
        candidates.push(...record.data);
      }
      if (Array.isArray(record.models)) {
        candidates.push(...record.models);
      }

      const nestedData = record.data;
      if (nestedData && typeof nestedData === "object") {
        const nestedRecord = nestedData as Record<string, unknown>;
        if (Array.isArray(nestedRecord.models)) {
          candidates.push(...nestedRecord.models);
        }
      }
    }

    const seen = new Set<string>();
    const models: { id: string; name: string }[] = [];

    for (const candidate of candidates) {
      let id = "";

      if (typeof candidate === "string") {
        id = candidate.trim();
      } else if (candidate && typeof candidate === "object") {
        const record = candidate as Record<string, unknown>;
        const rawId = [record.id, record.name, record.model]
          .find((value) => typeof value === "string" && value.trim().length > 0);

        if (typeof rawId === "string") {
          id = rawId.trim();
        }
      }

      if (!id || seen.has(id)) continue;
      seen.add(id);
      models.push({ id, name: id });
    }

    return models;
  }

  function buildModelsUrl(baseUrl: string) {
    let url = baseUrl.trim().replace(/\/+$/, "");

    if (!url) return url;

    // LM Studio serves the model list at /v1/models, not /api/v1/models.
    url = url.replace(/\/api(?=\/v1(?:\/models)?$)/, "");

    if (url.endsWith("/models")) {
      return url;
    }

    return url.endsWith("/v1") ? `${url}/models` : `${url}/v1/models`;
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
      const url = buildModelsUrl(baseUrl);
      const resp = await tauriFetch(url, {
        method: 'GET',
        headers: {
          Accept: 'application/json',
        },
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const responseText = await resp.text();
      let data: unknown = null;
      if (responseText.trim().length > 0) {
        try {
          data = JSON.parse(responseText);
        } catch {
          throw new Error("Invalid JSON response");
        }
      }

      const models = extractModelsFromPayload(data);

      if (models.length === 0) throw new Error("No models found");
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
  let progress = $state<TranslateProgressEvent | null>(null);
  let logs = $state<LogEntry[]>([]);
  let logIdCounter = 0;
  let error = $state<string | null>(null);
  let result = $state<TranslateResult | null>(null);
  let expandedPathField = $state<string | null>(null);

  let helpSection = $state<string | null>(null);

  // Live subtitle preview - array of translated subtitle pairs
  interface SubtitlePair {
    id: number;
    original: string;
    translated: string;
  }
  let translatedPairs = $state<SubtitlePair[]>([]);
  let previewRefreshInterval: ReturnType<typeof setInterval> | null = null;

  let apiKeys = $state<ApiKeyConfig[]>([]);

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
      { id: "google", name: t("provider.google") },
      { id: "groq", name: t("provider.groq") },
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

  let shouldKeepProviderPickerOpen = $derived(
    !providerConfirmed || (!hasValidKey && selectedProviderFamily !== "local"),
  );

  function handleStorageChange(e: StorageEvent) {
    if (e.key === "srt-tools-api-keys") {
      loadApiKeys();
    }
  }

  onMount(async () => {
    loadApiKeys();

    window.addEventListener("storage", handleStorageChange);

    // Also listen for custom event for same-window updates
    window.addEventListener("apikeys-updated", loadApiKeys);

    try {
      unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
        if (!active) return;
        if (event.payload.type === "over") isDraggingOver = true;
        else if (event.payload.type === "drop") {
          isDraggingOver = false;
          if (event.payload.paths) handleFileDrop(event.payload.paths);
        } else if (event.payload.type === "leave") isDraggingOver = false;
      });
    } catch (e) {
      console.warn("Failed to set up drag-drop listener:", e);
    }

    unlistenProgress = await listen<TranslateProgressEvent>(
      "translate-progress",
      (event) => {
        progress = event.payload;
        addLog(event.payload.message);
      },
    );

    unlistenComplete = await listen<TranslateResult>(
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
          addLog(`📁 Saved: ${event.payload.output_path.split("/").pop()}`);
        }
        addLog(`✅ ${event.payload.message} (${event.payload.translated_count} subtitles)`);
      },
    );
  });

  onDestroy(() => {
    window.removeEventListener("storage", handleStorageChange);
    window.removeEventListener("apikeys-updated", loadApiKeys);
    unlistenProgress?.();
    unlistenComplete?.();
    if (previewRefreshInterval) {
      clearInterval(previewRefreshInterval);
      previewRefreshInterval = null;
    }
    if (unlistenDragDrop) unlistenDragDrop();
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
        addLog(`📥 Input selected: ${inputPath.split("/").pop()}`);
        addLog(`📤 Output set: ${outputPath.split("/").pop()}`);
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
        addLog(`📤 Output updated: ${outputPath.split("/").pop()}`);
      }
    } catch (e) {
      error = `${t("translate.errorSelectingFile")} ${e}`;
    }
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
    if (!inputPath || !outputPath || !effectiveModel) {
      error = t("translate.selectFileAndKey");
      return;
    }

    if (!hasValidKey) {
      // Should not happen as button is disabled
      error = t("translate.noApiWarning");
      return;
    }

    error = null;
    result = null;
    progress = null;
    isTranslating = true;
    translatedPairs = [];
    addLog(`🚀 ${t("translate.starting")}`);
    addLog(`🌐 Target language: ${targetLang}`);
    addLog(`🧠 Provider: ${selectedProviderFamily} | Model: ${effectiveModel}`);
    addLog(`⚙️ Batch size: ${batchSize}, overlap: ${resumeOverlap}`);
    startPreviewRefresh();

    let keysToSend: string[] = [];
    if (selectedProviderFamily === "local") {
      keysToSend = []; // Local doesn't need API keys
    } else if (selectedProviderFamily === "google") {
      // Collect ALL Google keys (round-robin rotation)
      keysToSend = apiKeys
        .filter((k) => k.apiType === "google")
        .map((k) => k.apiKey.trim())
        .filter((k) => k.length > 0);
    } else if (selectedProviderFamily === "groq") {
      keysToSend = apiKeys
        .filter((k) => k.apiType === "groq")
        .map((k) => k.apiKey.trim())
        .filter((k) => k.length > 0);
    } else if (selectedProviderFamily === "custom" && selectedCustomProviderId) {
      const customKey = apiKeys.find((k) => k.id === selectedCustomProviderId);
      if (customKey && customKey.apiKey && customKey.apiKey.trim()) {
        keysToSend = [customKey.apiKey.trim()];
      }
    }

    if (keysToSend.length > 0) {
      addLog(`🔑 Using ${keysToSend.length} API key(s)`);
      const hasValidGoogleKeys = keysToSend.some((k) => k.startsWith("AIza"));
      if (!hasValidGoogleKeys && selectedProviderFamily === "google") {
        addLog(
          `⚠️ Warning: No valid Google AI keys found. Google API keys should start with 'AIza...'`,
        );
      }
    } else if (selectedProviderFamily !== "local") {
      addLog(`⚠️ No valid API keys found for ${selectedProviderFamily}`);
    }

    let apiUrl: string | null = null;
    if (selectedProviderFamily === "local") {
      apiUrl = localServerUrl || DEFAULT_LOCAL_URL;
    } else if (selectedProviderFamily === "google") {
      apiUrl = "https://generativelanguage.googleapis.com/v1beta";
    } else if (selectedProviderFamily === "groq") {
      apiUrl = "https://api.groq.com/openai/v1";
    } else if (selectedProviderFamily === "custom" && selectedCustomProviderId) {
      const customKey = apiKeys.find((k) => k.id === selectedCustomProviderId);
      if (customKey) {
        apiUrl = customKey.apiUrl || null;
      }
    }

    const config: TranslateConfig = {
      input_path: inputPath,
      output_path: outputPath,
      target_lang: targetLang,
      api_keys: keysToSend,
      api_type: selectedProviderFamily,
      batch_size: batchSize,
      resume_overlap: resumeOverlap,
      title_context: titleContext || null,
      api_url: apiUrl,
      model: effectiveModel || null,
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

  function handleGoToSettings() {
    if (onGoToSettings) {
      onGoToSettings();
    }
  }

  function showNoKeySnackbar(family: string) {
    if (tempSnackbarTimer) clearTimeout(tempSnackbarTimer);
    tempSnackbar = family;
    tempSnackbarTimer = setTimeout(() => {
      tempSnackbar = "";
      tempSnackbarTimer = null;
    }, 4000);
  }

  const TRANSLATE_PANEL_IDS = [
    "options",
    "files",
    "actions",
    "progress",
    "livePreview",
    "logs",
  ] as const;

  type TranslatePanelId = (typeof TRANSLATE_PANEL_IDS)[number];

  interface TranslateColumnLayout {
    col1: TranslatePanelId[];
    col2: TranslatePanelId[];
  }

  const TRANSLATE_DEFAULT_LAYOUT: TranslateColumnLayout = {
    col1: ["options", "logs"],
    col2: ["files", "actions", "progress", "livePreview"],
  };

  function loadTranslateLayout(): TranslateColumnLayout {
    try {
      const saved = localStorage.getItem("srt-translate-layout-v1");
      if (saved) {
        const parsed = JSON.parse(saved) as TranslateColumnLayout;
        const all = [...parsed.col1, ...parsed.col2];
        const valid =
          TRANSLATE_PANEL_IDS.every((id) => all.includes(id)) &&
          all.length === TRANSLATE_PANEL_IDS.length;
        if (valid) return parsed;
      }
    } catch {}
    return { ...TRANSLATE_DEFAULT_LAYOUT };
  }

  function saveTranslateLayout(layout: TranslateColumnLayout) {
    localStorage.setItem("srt-translate-layout-v1", JSON.stringify(layout));
  }

  let translatePanelLayout = $state<TranslateColumnLayout>(
    loadTranslateLayout(),
  );

  let trDraggedPanel = $state<TranslatePanelId | null>(null);
  let trDragOverCol = $state<"col1" | "col2" | null>(null);
  let trDragOverIdx = $state<number | null>(null);

  function trOnDragStart(e: DragEvent, panelId: TranslatePanelId) {
    const target = e.target as HTMLElement;
    if (
      target?.tagName === "INPUT" &&
      (target as HTMLInputElement).type === "range"
    ) {
      e.preventDefault();
      return;
    }
    trDraggedPanel = panelId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", panelId);
    }
  }

  function trOnDragOver(e: DragEvent, col: "col1" | "col2", idx: number) {
    if (!trDraggedPanel) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    trDragOverCol = col;
    trDragOverIdx = idx;
  }

  function trOnDragOverColumn(e: DragEvent, col: "col1" | "col2") {
    if (!trDraggedPanel) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    trDragOverCol = col;
    if (trDragOverIdx === null) {
      trDragOverIdx = translatePanelLayout[col].length;
    }
  }

  function trOnDrop(col: "col1" | "col2", idx: number) {
    if (!trDraggedPanel) return;
    const newLayout = { ...translatePanelLayout };
    for (const c of ["col1", "col2"] as const) {
      const i = newLayout[c].indexOf(trDraggedPanel);
      if (i !== -1) {
        newLayout[c] = [...newLayout[c]];
        newLayout[c].splice(i, 1);
        if (c === col && i < idx) idx--;
        break;
      }
    }
    newLayout[col] = [...newLayout[col]];
    newLayout[col].splice(idx, 0, trDraggedPanel);
    translatePanelLayout = newLayout;
    saveTranslateLayout(translatePanelLayout);
    trDraggedPanel = null;
    trDragOverCol = null;
    trDragOverIdx = null;
  }

  function trOnDropColumn(col: "col1" | "col2") {
    trOnDrop(col, translatePanelLayout[col].length);
  }

  function trOnDragEnd() {
    trDraggedPanel = null;
    trDragOverCol = null;
    trDragOverIdx = null;
  }

  function resetTranslateLayout() {
    translatePanelLayout = {
      col1: [...TRANSLATE_DEFAULT_LAYOUT.col1],
      col2: [...TRANSLATE_DEFAULT_LAYOUT.col2],
    };
    saveTranslateLayout(translatePanelLayout);
  }
</script>

<div 
  role="region"
  aria-label="Translate content"
  class="h-full flex flex-col p-6 overflow-y-auto overflow-x-hidden translate-tab-scroll relative"
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
          <InfoButton onclick={() => (helpSection = "options")} />
        </h3>
        <div class="space-y-4">
          <!-- Provider Selection -->
          <div>
            <span class="block text-sm text-gray-400 mb-2"
              >{t("translate.provider")}</span
            >

            {#if shouldKeepProviderPickerOpen}
              <!-- 2×2 Provider Grid -->
              <div class="grid grid-cols-2 gap-2">
                <!-- Local LLM -->
                <button
                  type="button"
                  onclick={() => {
                    selectedProviderFamily = "local";
                    providerConfirmed = true;
                  }}
                  class="flex items-center gap-2 p-2.5 rounded-lg transition-all duration-200 border text-left text-xs
                    {selectedProviderFamily === 'local'
                    ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}"
                >
                  <div
                    class="w-7 h-7 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-white shadow-lg flex-shrink-0"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                      ><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                      /></svg
                    >
                  </div>
                  <div class="flex flex-col min-w-0">
                    <span class="font-semibold truncate">{t("provider.local")}</span>
                    <span class="text-[9px] opacity-60 truncate">{t("provider.local.desc")}</span>
                  </div>
                </button>

                <!-- Provider Personalizzato -->
                <div class="relative group/provider">
                <button
                  type="button"
                  onclick={() => {
                    if (!hasCustomKey) { handleGoToSettings(); return; }
                    selectedProviderFamily = "custom";
                    providerConfirmed = true;
                  }}
                  class="w-full flex items-center gap-2 p-2.5 rounded-lg transition-all duration-200 border text-left text-xs
                    {selectedProviderFamily === 'custom'
                    ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}
                    {!hasCustomKey ? 'opacity-50 cursor-pointer hover:bg-white/5' : ''}"
                >
                  <div
                    class="w-7 h-7 rounded-lg bg-gradient-to-br from-gray-500 to-gray-600 flex items-center justify-center text-white shadow-lg flex-shrink-0 {!hasCustomKey ? 'grayscale' : ''}"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                      ><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                      /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                      /></svg
                    >
                  </div>
                  <div class="flex flex-col min-w-0">
                    <span class="font-semibold truncate">{t("provider.custom")}</span>
                    <span class="text-[9px] opacity-60 truncate">{t("provider.custom.desc")}</span>
                  </div>
                </button>
                {#if !hasCustomKey}
                  <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-gray-800 border border-white/10 text-xs text-amber-300 rounded-lg shadow-xl opacity-0 group-hover/provider:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
                    {t("translate.noKeyTooltip")}
                  </div>
                {/if}
                </div>

                <!-- Google Gemini -->
                <div class="relative group/provider">
                <button
                  type="button"
                  onclick={() => {
                    if (!hasGoogleKey) { handleGoToSettings(); return; }
                    selectedProviderFamily = "google";
                    providerConfirmed = true;
                  }}
                  class="w-full flex items-center gap-2 p-2.5 rounded-lg transition-all duration-200 border text-left text-xs
                    {selectedProviderFamily === 'google'
                    ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}
                    {!hasGoogleKey ? 'opacity-50 cursor-pointer hover:bg-white/5' : ''}"
                >
                  <div
                    class="w-7 h-7 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center text-white shadow-lg flex-shrink-0 {!hasGoogleKey ? 'grayscale' : ''}"
                  >
                    <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"
                      ><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                      /><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                      /><path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                      /><path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                      /></svg
                    >
                  </div>
                  <div class="flex flex-col min-w-0">
                    <span class="font-semibold truncate">{t("provider.google")}</span>
                    <span class="text-[9px] opacity-60 truncate">{t("provider.google.desc")}</span>
                  </div>
                </button>
                {#if !hasGoogleKey}
                  <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-gray-800 border border-white/10 text-xs text-amber-300 rounded-lg shadow-xl opacity-0 group-hover/provider:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
                    {t("translate.noKeyTooltip")}
                  </div>
                {/if}
                </div>

                <!-- Groq API -->
                <div class="relative group/provider">
                <button
                  type="button"
                  onclick={() => {
                    if (!hasGroqKey) { handleGoToSettings(); return; }
                    selectedProviderFamily = "groq";
                    providerConfirmed = true;
                  }}
                  class="w-full flex items-center gap-2 p-2.5 rounded-lg transition-all duration-200 border text-left text-xs
                    {selectedProviderFamily === 'groq'
                    ? 'bg-indigo-500/20 border-indigo-500/50 text-white'
                    : 'bg-white/5 hover:bg-white/10 border-transparent text-gray-400 hover:text-white'}
                    {!hasGroqKey ? 'opacity-50 cursor-pointer hover:bg-white/5' : ''}"
                >
                  <div
                    class="w-7 h-7 rounded-lg bg-gradient-to-br from-orange-400 to-red-500 flex items-center justify-center text-white shadow-lg flex-shrink-0 {!hasGroqKey ? 'grayscale' : ''}"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                      ><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M13 10V3L4 14h7v7l9-11h-7z"
                      /></svg
                    >
                  </div>
                  <div class="flex flex-col min-w-0">
                    <span class="font-semibold truncate">{t("provider.groq")}</span>
                    <span class="text-[9px] opacity-60 truncate">{t("provider.groq.desc")}</span>
                  </div>
                </button>
                {#if !hasGroqKey}
                  <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-gray-800 border border-white/10 text-xs text-amber-300 rounded-lg shadow-xl opacity-0 group-hover/provider:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
                    {t("translate.noKeyTooltip")}
                  </div>
                {/if}
                </div>
              </div>

            {:else}
              <!-- Confirmed Provider: single card + change button -->
              <div class="flex items-center gap-2">
                <div class="flex items-center gap-2 p-2.5 rounded-lg bg-indigo-500/20 border border-indigo-500/50 text-white text-xs flex-1">
                  <div class="w-7 h-7 rounded-lg bg-gradient-to-br flex items-center justify-center text-white shadow-lg flex-shrink-0
                    {selectedProviderFamily === 'local' ? 'from-purple-500 to-pink-500' :
                     selectedProviderFamily === 'custom' ? 'from-gray-500 to-gray-600' :
                     selectedProviderFamily === 'google' ? 'from-blue-500 to-cyan-500' :
                     'from-orange-400 to-red-500'}">
                    {#if selectedProviderFamily === 'local'}
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/></svg>
                    {:else if selectedProviderFamily === 'custom'}
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
                    {:else if selectedProviderFamily === 'google'}
                      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"><path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"/><path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/><path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/><path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/></svg>
                    {:else}
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
                    {/if}
                  </div>
                  <span class="font-semibold truncate">
                    {providerOptions.find(p => p.id === selectedProviderFamily)?.name || selectedProviderFamily}
                  </span>
                </div>
                <button
                  type="button"
                  onclick={() => { providerConfirmed = false; selectedProviderFamily = ""; selectedModel = ""; localCustomModel = ""; }}
                  class="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-400 hover:text-white transition-all border border-transparent hover:border-white/10"
                  title={t("translate.changeProvider")}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"/>
                  </svg>
                </button>
              </div>
            {/if}
          </div>

          <!-- Custom Provider Select (only when custom is confirmed) -->
          {#if providerConfirmed && selectedProviderFamily === "custom"}
            <div>
              <label for="custom-provider-select" class="block text-sm text-gray-400 mb-1"
                >{t("translate.selectCustomProvider")}</label
              >
              {#if savedCustomProviders.length > 0}
                <select
                  id="custom-provider-select"
                  bind:value={selectedCustomProviderId}
                  class="w-full bg-black/40 border border-white/10 rounded-lg px-4 py-2.5 text-sm text-white focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500/50 outline-none transition-all"
                >
                  <option value="">{t("translate.selectCustomProvider")}</option>
                  {#each savedCustomProviders as cp}
                    <option value={cp.id}>{cp.name}</option>
                  {/each}
                </select>
              {:else}
                <p class="text-xs text-gray-500 italic">
                  {t("translate.noCustomProviders")}
                  <button
                    type="button"
                    onclick={handleGoToSettings}
                    class="underline hover:text-cyan-400 transition-colors"
                  >
                    {t("translate.goToSettings")}
                  </button>
                </p>
              {/if}
            </div>
          {/if}

          <!-- Local Server URL (shown when local provider is confirmed) -->
          {#if providerConfirmed && selectedProviderFamily === "local"}
            <div>
              <label for="local-server-url" class="block text-sm text-gray-400 mb-1">{t("translate.localServerUrl")}</label>
              <div class="flex items-center gap-2">
                <input
                  id="local-server-url"
                  type="text"
                  value={localServerUrl}
                  oninput={(e) => saveLocalServerUrl((e.target as HTMLInputElement).value)}
                  placeholder={DEFAULT_LOCAL_URL}
                  class="input-modern flex-1 text-sm font-mono"
                />
                <button
                  type="button"
                  onclick={() => fetchModelsFromServer(localServerUrl)}
                  disabled={isFetchingModels || !localServerUrl.trim()}
                  class="px-3 py-2 rounded-lg text-xs font-medium transition-all
                    {isFetchingModels ? 'bg-white/5 text-gray-500 cursor-wait' : 'bg-indigo-500/20 hover:bg-indigo-500/30 text-indigo-300 border border-indigo-500/30 hover:border-indigo-500/50'}"
                >
                  {#if isFetchingModels}
                    <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
                  {:else}
                    {t("translate.fetchModels")}
                  {/if}
                </button>
              </div>
              {#if fetchModelsError}
                <p class="text-[10px] text-amber-400 mt-1">⚠ {fetchModelsError}</p>
              {:else if fetchedModels.length > 0}
                <p class="text-[10px] text-emerald-400 mt-1">✓ {fetchedModels.length} {t("translate.modelsFound")}</p>
              {/if}
            </div>

            <!-- Fetch models for custom provider -->
            {#if selectedCustomProviderId}
              {@const customProviderEntry = apiKeys.find((k) => k.id === selectedCustomProviderId)}
              {#if customProviderEntry?.apiUrl}
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    onclick={() => fetchModelsFromServer(customProviderEntry.apiUrl!)}
                    disabled={isFetchingModels}
                    class="px-3 py-2 rounded-lg text-xs font-medium transition-all
                      {isFetchingModels ? 'bg-white/5 text-gray-500 cursor-wait' : 'bg-indigo-500/20 hover:bg-indigo-500/30 text-indigo-300 border border-indigo-500/30 hover:border-indigo-500/50'}"
                  >
                    {#if isFetchingModels}
                      <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/></svg>
                    {:else}
                      {t("translate.fetchModels")}
                    {/if}
                  </button>
                  {#if fetchModelsError}
                    <span class="text-[10px] text-amber-400">⚠ {fetchModelsError}</span>
                  {:else if fetchedModels.length > 0}
                    <span class="text-[10px] text-emerald-400">✓ {fetchedModels.length} {t("translate.modelsFound")}</span>
                  {/if}
                </div>
              {/if}
            {/if}
          {/if}

          <!-- Model selector (shown for all confirmed providers) -->
          {#if providerConfirmed && (selectedProviderFamily !== "custom" || (selectedCustomProviderId && availableModels.length > 0))}
            <div>
              <div class="flex items-center justify-between mb-1">
                <label for="model-select" class="block text-sm text-gray-400"
                  >{t("translate.model")}</label
                >
                {#if selectedProviderFamily === "google" || selectedProviderFamily === "groq" || selectedProviderFamily === "local" || selectedProviderFamily === "custom"}
                  <button
                    type="button"
                    onclick={() => {
                        if (selectedProviderFamily === "local" || selectedProviderFamily === "custom") {
                            const url = selectedProviderFamily === "local" ? localServerUrl : apiKeys.find((k) => k.id === selectedCustomProviderId)?.apiUrl;
                            if (url) fetchModelsFromServer(url, true);
                        } else {
                            fetchProviderModels(selectedProviderFamily, true);
                        }
                    }}
                    disabled={isFetchingModels}
                    class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors flex items-center gap-1"
                    title={t("translate.refetchModelsTooltip")}
                  >
                    <svg class="w-3 h-3 {isFetchingModels ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                    Refresh
                  </button>
                {/if}
              </div>
              {#if availableModels.length === 0 && (selectedProviderFamily === "local" || selectedProviderFamily === "custom")}
                <p class="text-xs text-gray-500 italic py-2">{t("translate.fetchModelsHint")}</p>
              {/if}
              {#if availableModels.length > 0}
                <SearchableSelect
                  noResultsText={t("common.noResults")}
                  options={availableModels.map((m) => ({
                    value: m.id,
                    label: m.name,
                  }))}
                  value={selectedModel}
                  onchange={(v) => {
                    selectedModel = v;
                    localCustomModel = "";
                  }}
                  placeholder={t("translate.model")}
                />
              {/if}

              <!-- Local/Custom provider: allow typing arbitrary model IDs -->
              {#if selectedProviderFamily === "local" || selectedProviderFamily === "custom"}
                {#if localCustomModel.trim()}
                  <div class="flex items-center gap-2 mt-1.5">
                    <input
                      type="text"
                      bind:value={localCustomModel}
                      placeholder={t("translate.localCustomModelPlaceholder")}
                      class="input-modern flex-1 text-sm font-mono"
                    />
                    <button
                      type="button"
                      onclick={() => (localCustomModel = "")}
                      class="text-xs text-gray-400 hover:text-white px-2 py-1.5 rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
                      title={t("translate.model")}>✕</button
                    >
                  </div>
                  <p class="text-[10px] text-emerald-400 mt-1">
                    ✓ {t("translate.usingCustomModel")}:
                    <strong>{localCustomModel.trim()}</strong>
                  </p>
                {/if}
              {/if}
            </div>
          {/if}

          <!-- Custom provider: manual model input when no models fetched -->
          {#if providerConfirmed && selectedProviderFamily === "custom" && selectedCustomProviderId && availableModels.length === 0}
            <div>
              <label for="custom-model-input" class="block text-sm text-gray-400 mb-1">{t("translate.model")}</label>
              <input
                id="custom-model-input"
                type="text"
                bind:value={localCustomModel}
                placeholder={t("translate.customModelPlaceholder")}
                class="input-modern w-full text-sm font-mono"
              />
            </div>
          {/if}

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
                  searchTerms: `${lang.nameEn} ${lang.name}`,
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
              <p class="text-[10px] text-gray-500 mt-1">Number of previously translated subtitles to re-send to the LLM for context when resuming.</p>
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
          class="text-lg font-semibold mb-4 flex items-center gap-2 text-indigo-400"
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
          {t("translate.file")}
          <InfoButton onclick={() => (helpSection = "files")} />
        </h3>
        <div class="space-y-3">
          <PathPickerField
            label={t("translate.inputFile")}
            value={inputPath}
            placeholder={t("translate.selectFile")}
            browseTitle={t("translate.tooltip.upload")}
            onexpand={() => (expandedPathField = "input")}
            onbrowse={selectInputFile}
            browseButtonClass="btn-primary py-2 px-3"
            browseIconPath="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
          />

          <PathPickerField
            label={t("translate.outputFile")}
            value={outputPath}
            placeholder={t("translate.selectDestination")}
            browseTitle={t("translate.tooltip.save")}
            onexpand={() => (expandedPathField = "output")}
            onbrowse={selectOutputFile}
            browseButtonClass="btn-secondary py-2 px-3"
            browseIconPath="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
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
                  <p class="text-sm text-gray-400 truncate max-w-xs">
                    "{fileInfo.first_subtitle}"
                  </p>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else if panelId === "actions"}
      <div class="flex gap-3">
        {#if isTranslating}
          <button
            onclick={cancelTranslation}
            class="btn-danger flex-1 py-4 text-lg"
          >
            <svg
              class="w-5 h-5 inline mr-2"
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
          <button
            onclick={resetTranslation}
            class="btn-secondary flex-1 py-4 text-lg bg-indigo-500/20 text-indigo-300 border border-indigo-500/30 hover:bg-indigo-500/30 transition-all font-semibold"
          >
            <svg class="w-5 h-5 inline mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            New Translation
          </button>
        {:else}
          <button
            onclick={startTranslation}
            disabled={!inputPath ||
              !outputPath ||
              !hasValidKey ||
              (selectedProviderFamily !== "custom" && !selectedModel)}
            class="btn-success flex-1 py-4 text-lg disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none"
          >
            <svg
              class="w-5 h-5 inline mr-2"
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
        {/if}
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
      <div class="glass-card p-5 shrink-0" style="min-height: 400px;">
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
        <div class="grid grid-cols-2 gap-4" style="min-height: 340px;">
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

  <div class="flex-1 grid grid-cols-1 xl:grid-cols-2 gap-6 min-h-0 overflow-y-auto transition-opacity">
    <div
      class="space-y-3 overflow-y-auto overflow-x-hidden pr-1 min-h-[100px]"
      role="list"
    >
      {#each translatePanelLayout.col1 as trPanelId, idx (trPanelId)}
        <div
          class="relative transition-all duration-150 flex-1 flex flex-col"
          role="listitem"
        >
          {@render panelContent(trPanelId)}
        </div>
      {/each}
    </div>

    <div
      class="space-y-3 overflow-y-auto overflow-x-hidden pr-1 min-h-[100px]"
      role="list"
    >
      {#each translatePanelLayout.col2 as trPanelId, idx (trPanelId)}
        <div
          class="transition-all duration-150"
          role="listitem"
        >
          {@render panelContent(trPanelId)}
        </div>
      {/each}
    </div>
  </div>

  <InfoModal 
    section={helpSection} 
    sections={translateSections} 
    onclose={() => (helpSection = null)} 
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "input"
      ? t("translate.inputFile")
      : t("translate.outputFile")}
    value={expandedPathField === "input" ? inputPath : outputPath}
    onclose={() => (expandedPathField = null)}
  />
</div>

<style>
  .translate-tab-scroll {
    background: #111827;
    contain: layout style;
    -webkit-overflow-scrolling: touch;
    overflow-y: auto;
    overscroll-behavior: contain;
    transform: translateZ(0);
    will-change: scroll-position;
    backface-visibility: hidden;
  }
</style>

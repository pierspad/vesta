<script lang="ts">
  import { LatestRequest } from "$lib/utils/asyncControl.svelte";
  const downloadedRequests = new LatestRequest();
  const frameRequests = new LatestRequest();
  let disposed = false;
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import SettingsJumpButton from "$lib/components/SettingsJumpButton.svelte";
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import LoadingSpinner from "$lib/components/LoadingSpinner.svelte";
  import ProgressFill from "$lib/components/ProgressFill.svelte";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { onMount, onDestroy, untrack } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { SubtitlePreviewSession } from "$lib/utils/subtitlePreviewSession";
  import { join } from "@tauri-apps/api/path";
  import { defaultOutputDirectory } from "$lib/utils/defaultOutputDirectory";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { ocrModelsStore } from "$lib/stores/ocrModelsStore.svelte";
  import { ocrHardcodedSubtitle, ocrVideoPreviewFrame, previewOcrHardcodedSubtitle, type VideoFramePreview, type VideoRoi } from "$lib/services/ocr";
  import { loadTiers } from "$lib/config/translationTiers";
  import { loadIttTiers } from "$lib/config/ittTiers";
  import { loadAndValidateApiKeys } from "$lib/config/apiKeys";
  import { buildTiersPayload, type TierEntryPayload } from "$lib/config/llmTiers";
  import { languages as allLanguages, getLanguageSearchTerms } from "$lib/config/languages";
  import * as vestaConfig from "$lib/config/vestaConfig";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import { prepareSubtitleTracks, filterPreparedSubtitleTracks, subtitleOutputName, groupSubtitleTracks, describeSubtitleTrack, SUBTITLE_PAGE_SIZE } from "$lib/utils/subtitleTracks";
  import SubtitlePreviewModal from "$lib/modals/SubtitlePreviewModal.svelte";
  import SubtitleTrackCard from "$lib/components/SubtitleTrackCard.svelte";
  import { locale, currentLanguage } from "$lib/i18n";
  import { errorMessage } from "$lib/utils/errorMessage";
  let { active = false, onGoToSettings }: {
    active?: boolean;
    onGoToSettings?: (section?: "overview" | "llm" | "whisper" | "itt" | "language" | "anki" | "shortcuts", highlightItemId?: string) => void;
  } = $props();
  let isDraggingOver = $state(false);
  $effect(() => {
    if (!active) { isDraggingOver = false; return; }
    return setupWebviewDragDrop({
      isActive: () => active && !busy && downloadingIndex === null && !extractingHardcoded && !previewOpen,
      setDraggingOver: value => isDraggingOver = value,
      onDrop: paths => {
        const media = paths.find(value => /\.(mkv|mp4|m4v|avi|mov|webm|ts|mp3|flac|m4a|wav|ogg|aac)$/i.test(value));
        if (media) void loadMedia(media);
      },
    });
  });
  let t = $derived($locale);
  interface Track { index: number; codec: string; language: string; title: string; text_based: boolean }
  let tracks = $state<Track[]>([]);
  let tracksScanned = $state(false);
  let search = $state("");
  let page = $state(0);
  let hasLocalOcr = $derived(ocrModelsStore.downloadedOcrCount > 0);
  let canOcr = $derived(!aiStore.killSwitchActive || hasLocalOcr);
  let prepared = $derived(prepareSubtitleTracks(tracks, $currentLanguage));
  let filtered = $derived(filterPreparedSubtitleTracks(prepared, search));
  let groups = $derived(groupSubtitleTracks(filtered, $currentLanguage));
  let pageCount = $derived(Math.max(1, Math.ceil(groups.length / SUBTITLE_PAGE_SIZE)));
  let visible = $derived(groups.slice(page * SUBTITLE_PAGE_SIZE, (page + 1) * SUBTITLE_PAGE_SIZE));
  $effect(() => { if (page >= pageCount) page = pageCount - 1; });
  let outputDir = $state("");
  let downloaded = $state<number[]>([]);
  let selections = $state<Record<string, number>>({});
  let previewGroups = $derived(groupSubtitleTracks(prepared, $currentLanguage));
  let previewVariants = $derived(previewGroups.find(group => group.tracks.some(track => track.index === previewTrack?.index))?.tracks.filter(track => track.text_based || canOcr) ?? []);
  let previewPosition = $derived(previewVariants.findIndex(track => track.index === previewTrack?.index));
  onMount(() => {
    void ocrModelsStore.refreshModels().catch(() => {});
    void defaultOutputDirectory().then(value => { if (!disposed && !outputDir) { outputDir = value; void refreshDownloaded(); } }).catch(e => { if (!disposed) error = errorMessage(e); });
    const refresh = () => { void refreshDownloaded(); };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  });
  async function refreshDownloaded() {
    if (disposed) return;
    const request = downloadedRequests.begin();
    const directory = outputDir;
    const media = path;
    const candidates = tracks;
    if (!directory || !media) { downloaded = []; return; }
    try {
      const names = await invoke<string[]>("existing_subtitle_outputs", { directory, names: candidates.map(track => subtitleOutputName(media, track)) });
      if (request.isCurrent() && directory === outputDir && media === path) downloaded = candidates.filter(track => names.includes(subtitleOutputName(media, track))).map(track => track.index);
    } catch (e) {
      if (request.isCurrent() && directory === outputDir && media === path) error = errorMessage(e);
    }
  }
  async function chooseOutput() {
    if (disposed || busy || downloadingIndex !== null || extractingHardcoded) return;
    busy = true;
    try {
      const selected = await guardedOpen({ directory: true, multiple: false, defaultPath: outputDir || undefined });
      if (disposed) return;
      if (typeof selected === "string") { outputDir = selected; downloaded = []; await refreshDownloaded(); }
    } catch (e) { if (!disposed) error = errorMessage(e); }
    finally { if (!disposed) busy = false; }
  }
  function selectPreview() {
    if (previewMode === "hardcoded") {
      closePreview();
      return;
    }
    const group = previewGroups.find(group => group.tracks.some(track => track.index === previewTrack?.index));
    if (group && previewTrack) selections = { ...selections, [group.key]: previewTrack.index };
    closePreview();
  }
  interface OcrProgressPayload {
    stage: string;
    message: string;
    percentage: number;
    current: number;
    total: number;
  }
  let previewTrack = $state<Track | null>(null);
  let previewMode = $state<"track" | "hardcoded">("track");
  let previewOpen = $derived(Boolean(previewTrack || previewMode === "hardcoded"));
  let previewText = $state("");
  let previewError = $state("");
  let previewLoading = $state(false);
  let previewSession: SubtitlePreviewSession | null = null;
  let previewRequest = 0;
  let ocrProgress = $state<{ percentage: number; current: number; total: number; message: string } | null>(null);
  let ocrCancelling = $state(false);
  const ocrPreviewCache = new Map<number, string>();
  let ocrUnlisten: UnlistenFn | null = null;
  let ownsOcr = false;
  let activeOcrUsesCloud = false;

  const ROI_PRESET_BOTTOM: VideoRoi = { x: 0.08, y: 0.74, width: 0.84, height: 0.20 };
  const ROI_PRESET_TOP: VideoRoi = { x: 0.08, y: 0.05, width: 0.84, height: 0.20 };
  const ROI_PRESET_FULL: VideoRoi = { x: 0.02, y: 0.02, width: 0.96, height: 0.96 };
  const MIN_ROI_SIZE = 0.04;
  type RoiResizeHandle = "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se";
  const ROI_HANDLES: { handle: RoiResizeHandle; className: string }[] = [
    { handle: "nw", className: "-left-1.5 -top-1.5 h-3 w-3 rounded-full border-2 cursor-nwse-resize" },
    { handle: "ne", className: "-right-1.5 -top-1.5 h-3 w-3 rounded-full border-2 cursor-nesw-resize" },
    { handle: "sw", className: "-bottom-1.5 -left-1.5 h-3 w-3 rounded-full border-2 cursor-nesw-resize" },
    { handle: "se", className: "-bottom-1.5 -right-1.5 h-3 w-3 rounded-full border-2 cursor-nwse-resize" },
    { handle: "n", className: "-top-1 left-1/2 h-2 w-8 -translate-x-1/2 rounded-full border cursor-ns-resize" },
    { handle: "s", className: "-bottom-1 left-1/2 h-2 w-8 -translate-x-1/2 rounded-full border cursor-ns-resize" },
    { handle: "w", className: "-left-1 top-1/2 h-8 w-2 -translate-y-1/2 rounded-full border cursor-ew-resize" },
    { handle: "e", className: "-right-1 top-1/2 h-8 w-2 -translate-y-1/2 rounded-full border cursor-ew-resize" },
  ];

  let extractMode = $state<"embedded" | "hardcoded">("embedded");
  let roi = $state<VideoRoi>({ ...ROI_PRESET_BOTTOM });
  let sampleFps = $state(2.5);
  let hardcodedLanguage = $state(vestaConfig.getItem("vesta-default-transcribe-language") || "auto");
  let framePreview = $state<VideoFramePreview | null>(null);
  let frameTimeMs = $state<number | null>(null);
  let loadingFrame = $state(false);
  let extractingHardcoded = $state(false);
  let roiContainerEl = $state<HTMLDivElement | null>(null);
  let roiDragState = $state<{
    mode: "move" | "draw" | "resize";
    handle?: RoiResizeHandle;
    startX: number;
    startY: number;
    originRoi: VideoRoi;
  } | null>(null);

  const fpsOptions = [
    { value: "1", label: "1 FPS" },
    { value: "1.5", label: "1.5 FPS" },
    { value: "2", label: "2 FPS" },
    { value: "2.5", label: "2.5 FPS" },
    { value: "3", label: "3 FPS" },
    { value: "4", label: "4 FPS" },
    { value: "5", label: "5 FPS" },
  ];

  let hardcodedLanguageOptions = $derived([
    { value: "auto", label: t("transcribe.autoDetect"), icon: "🌐" },
    ...allLanguages.map((lang) => ({
      value: lang.code,
      label: lang.name,
      icon: lang.flag,
      searchTerms: getLanguageSearchTerms(lang.code),
    })),
  ]);

  function ocrLanguageForTrack(track: Track): string | null {
    return describeSubtitleTrack(track).code ?? (track.language && track.language !== "und" ? track.language : null);
  }
  function buildLocalOcrEntry(): TierEntryPayload | null {
    const localModel =
      ocrModelsStore.ocrModels.find((model) => model.id === ocrModelsStore.defaultOcrModel && model.downloaded) ??
      ocrModelsStore.ocrModels.find((model) => model.downloaded);
    if (!localModel) return null;
    return {
      provider: "local_ocr",
      model: localModel.id,
      api_key: null,
      api_url: null,
      rpm: null,
      max_requests: null,
    };
  }
  function tiersHaveCloud(tiers: TierEntryPayload[][]): boolean {
    return tiers.some((tier) => tier.some((entry) => entry.provider !== "local_ocr"));
  }
  function requireOcrTiers(): TierEntryPayload[][] | null {
    const localEntry = buildLocalOcrEntry();
    if (aiStore.killSwitchActive) {
      if (localEntry) return [[localEntry]];
      error = t("extract.ocrNoProvider");
      snackbar.show(error, "error");
      return null;
    }
    const keys = loadAndValidateApiKeys();
    const ittTiers = buildTiersPayload(loadIttTiers(), keys, "itt");
    const visionTiers = ittTiers ?? buildTiersPayload(loadTiers(), keys, "llm");
    if (visionTiers && localEntry) return [...visionTiers, [localEntry]];
    if (visionTiers) return visionTiers;
    if (localEntry) return [[localEntry]];
    error = t("extract.ocrNoProvider");
    snackbar.show(error, "error");
    return null;
  }
  function isPreviewOcrActive(): boolean {
    return Boolean(previewLoading && (previewMode === "hardcoded" || (previewTrack && !previewTrack.text_based)));
  }
  function closePreview() {
    const cancelOcrPreview = isPreviewOcrActive();
    previewRequest += 1;
    previewSession?.close();
    previewSession = null;
    previewTrack = null;
    previewMode = "track";
    previewText = "";
    previewError = "";
    previewLoading = false;
    if (cancelOcrPreview) {
      void invoke("ocr_cancel").catch(() => {});
    }
  }
  async function cancelOcr() {
    if (ocrCancelling || !aiStore.isOcring) return;
    ocrCancelling = true;
    try {
      await invoke("ocr_cancel");
    } catch (e) {
      if (!disposed) {
        ocrCancelling = false;
        snackbar.show(errorMessage(e), "error");
      }
    }
  }
  $effect(() => {
    if (aiStore.killSwitchActive && aiStore.isOcring) {
      untrack(() => {
        if (!ownsOcr || !activeOcrUsesCloud) return;
        if (isPreviewOcrActive()) {
          closePreview();
        } else {
          void cancelOcr();
        }
      });
    }
  });
  onDestroy(() => {
    disposed = true;
    stopRoiWindowListeners();
    downloadedRequests.invalidate();
    frameRequests.invalidate();
    if (ocrUnlisten) {
      try { ocrUnlisten(); } catch { /* ignore */ }
      ocrUnlisten = null;
    }
    const wasPreviewOcring = isPreviewOcrActive();
    closePreview();
    if (ownsOcr) {
      if (!wasPreviewOcring) void invoke("ocr_cancel").catch(() => {});
      ownsOcr = false;
      activeOcrUsesCloud = false;
      aiStore.isOcring = false;
    }
    ocrPreviewCache.clear();
  });
  async function runOcrPreview(
    mode: "track" | "hardcoded",
    track: Track | null,
    tiers: TierEntryPayload[][],
    fetchPreview: (media: string) => Promise<string>,
    onSuccess?: (text: string) => void,
  ) {
    error = "";
    const media = path;
    const request = ++previewRequest;
    previewMode = mode;
    previewTrack = track;
    previewText = "";
    previewError = "";
    previewLoading = true;
    activeOcrUsesCloud = tiersHaveCloud(tiers);
    ownsOcr = true;
    aiStore.isOcring = true;
    try {
      const text = await fetchPreview(media);
      if (!disposed && request === previewRequest && path === media) {
        onSuccess?.(text);
        previewText = text;
      }
    } catch (e) {
      if (!disposed && request === previewRequest) {
        previewError = errorMessage(e);
      }
    } finally {
      releaseOcrOwnership();
      if (!disposed && request === previewRequest) {
        previewLoading = false;
      }
      if (!disposed) {
        ocrCancelling = false;
      }
    }
  }
  function releaseOcrOwnership() {
    if (ownsOcr) {
      ownsOcr = false;
      activeOcrUsesCloud = false;
      aiStore.isOcring = false;
    }
  }
  function reportOcrOrExtractError(e: unknown, isOcr: boolean) {
    if (disposed) return;
    if (isOcr && (ocrCancelling || /cancel|annullat/i.test(errorMessage(e, "")))) {
      snackbar.show(t("extract.ocrCancelled"), "info");
    } else {
      error = errorMessage(e);
      snackbar.show(error, "error");
    }
  }
  async function preview(track: Track) {
    if (busy || !track) return;
    if (isPreviewOcrActive()) return;
    if (!track.text_based) {
      if (!canOcr || aiStore.isOcring) return;
      const cached = ocrPreviewCache.get(track.index);
      if (cached !== undefined) {
        error = "";
        previewRequest += 1;
        previewMode = "track";
        previewTrack = track;
        previewText = cached;
        previewError = "";
        previewLoading = false;
        return;
      }
      const tiers = requireOcrTiers();
      if (!tiers) return;
      await runOcrPreview(
        "track",
        track,
        tiers,
        (media) => invoke<string>("preview_ocr_embedded_subtitle", {
          path: media,
          index: track.index,
          language: ocrLanguageForTrack(track),
          tiers,
        }),
        (text) => ocrPreviewCache.set(track.index, text),
      );
      return;
    }
    if (!previewSession) {
      const media = path;
      const variants = previewGroups.find(group => group.tracks.some(variant => variant.index === track.index))?.tracks.filter(variant => variant.text_based) ?? [track];
      previewSession = new SubtitlePreviewSession(variants.map(variant => variant.index), index => invoke<string>("preview_embedded_subtitle", { path: media, index }));
    }
    const session = previewSession;
    const request = ++previewRequest;
    previewMode = "track";
    previewTrack = track; previewText = ""; previewError = ""; previewLoading = true;
    try {
      const text = await session.select(track.index);
      if (request === previewRequest) previewText = text;
    } catch (e) { if (request === previewRequest) previewError = errorMessage(e); }
    finally { if (request === previewRequest) previewLoading = false; }
  }
  let path = $state("");
  let isVideoMedia = $derived(/\.(mkv|mp4|m4v|avi|mov|webm|ts)$/i.test(path));
  let busy = $state(false);
  let downloadingIndex = $state<number | null>(null);
  let error = $state("");

  function resetMediaState() {
    closePreview();
    ocrPreviewCache.clear();
    frameRequests.invalidate();
    selections = {};
    downloaded = [];
    tracks = [];
    tracksScanned = false;
    search = "";
    page = 0;
    framePreview = null;
    frameTimeMs = null;
    loadingFrame = false;
    roiDragState = null;
  }

  async function resolveOutputDir(): Promise<string> {
    if (outputDir) return outputDir;
    const directory = await defaultOutputDirectory();
    if (!disposed) outputDir = directory;
    return directory;
  }

  async function attachOcrProgressListener(mediaPath: string): Promise<() => void> {
    let localUnlisten: UnlistenFn | null = null;
    try {
      const unlistenFn = await listen<OcrProgressPayload>("ocr-progress", (event) => {
        if (disposed || path !== mediaPath) return;
        const payload = event?.payload;
        if (!payload) return;
        ocrProgress = {
          percentage: payload.percentage,
          current: payload.current,
          total: payload.total,
          message: payload.message,
        };
      });
      if (typeof unlistenFn === "function") {
        if (disposed) {
          unlistenFn();
        } else {
          localUnlisten = unlistenFn;
          ocrUnlisten = unlistenFn;
        }
      }
    } catch {
      // Ignore listener registration errors in mocked or non-Tauri environments.
    }
    return () => {
      if (localUnlisten) {
        try { localUnlisten(); } catch { /* ignore */ }
        if (ocrUnlisten === localUnlisten) ocrUnlisten = null;
        localUnlisten = null;
      }
    };
  }

  async function chooseMedia() {
    if (disposed || busy || downloadingIndex !== null || extractingHardcoded) return;
    busy = true; error = "";
    try {
      const selected = await guardedOpen({ multiple: false, filters: [{ name: t("extract.media"), extensions: ["mkv", "mp4", "m4v", "avi", "mov", "webm", "ts"] }] });
      if (disposed || typeof selected !== "string") return;
      await loadMedia(selected, true);
    }
    catch (e) { if (!disposed) error = errorMessage(e); }
    finally { if (!disposed) busy = false; }
  }
  async function loadMedia(selected: string, fromPicker = false) {
    if (disposed || (!fromPicker && busy) || downloadingIndex !== null || extractingHardcoded) return;
    busy = true; error = ""; isDraggingOver = false;
    resetMediaState();
    path = selected;
    try {
      const loadedTracks = await invoke<Track[]>("embedded_subtitle_tracks", { path: selected });
      if (disposed) return;
      tracks = loadedTracks;
      tracksScanned = true;
      await refreshDownloaded();
      if (!disposed && extractMode === "hardcoded" && /\.(mkv|mp4|m4v|avi|mov|webm|ts)$/i.test(selected)) {
        void loadPreviewFrame();
      }
    } catch (e) { if (!disposed) error = errorMessage(e); }
    finally { if (!disposed) busy = false; }
  }
  async function extract(track: Track) {
    if (disposed || busy || downloadingIndex !== null || extractingHardcoded) return;
    let tiers: TierEntryPayload[][] | null = null;
    if (!track.text_based) {
      if (!canOcr || aiStore.isOcring) return;
      tiers = requireOcrTiers();
      if (!tiers) return;
    }
    error = "";
    downloadingIndex = track.index;
    ocrProgress = null;
    ocrCancelling = false;
    let detachListener: (() => void) | null = null;
    if (!track.text_based && tiers) {
      activeOcrUsesCloud = tiersHaveCloud(tiers);
      ownsOcr = true;
      aiStore.isOcring = true;
    }
    try {
      const directory = await resolveOutputDir();
      if (disposed) return;
      const outputPath = await join(directory, subtitleOutputName(path, track));
      if (disposed) return;
      if (!track.text_based) {
        detachListener = await attachOcrProgressListener(path);
        if (disposed) return;
        if (ocrCancelling) {
          snackbar.show(t("extract.ocrCancelled"), "info");
          return;
        }
        await invoke("ocr_embedded_subtitle", {
          path,
          index: track.index,
          outputPath,
          language: ocrLanguageForTrack(track),
          tiers,
        });
        if (disposed) return;
        if (ocrCancelling) {
          snackbar.show(t("extract.ocrCancelled"), "info");
          return;
        }
      } else {
        await invoke("extract_embedded_subtitle", { path, index: track.index, outputPath });
        if (disposed) return;
      }
      await refreshDownloaded();
      if (disposed) return;
      snackbar.show(`${t("extract.saved")} ${outputPath}`, "success");
    } catch (e) {
      reportOcrOrExtractError(e, !track.text_based);
    } finally {
      detachListener?.();
      releaseOcrOwnership();
      if (!disposed) {
        downloadingIndex = null;
        ocrProgress = null;
        ocrCancelling = false;
      }
    }
  }

  function roundRoiValue(value: number): number {
    return Math.round(value * 1000) / 1000;
  }

  function clamp01(value: number): number {
    return Math.max(0, Math.min(1, value));
  }

  function setRoiPreset(preset: "bottom" | "top" | "full") {
    roi = preset === "bottom" ? { ...ROI_PRESET_BOTTOM } : preset === "top" ? { ...ROI_PRESET_TOP } : { ...ROI_PRESET_FULL };
  }

  function centerRoiHorizontally() {
    roi = { ...roi, x: roundRoiValue(clamp01((1 - roi.width) / 2)) };
  }

  function updateRoiPercent(field: keyof VideoRoi, percent: number) {
    if (!Number.isFinite(percent)) return;
    const value = clamp01(percent / 100);
    if (field === "x") {
      const width = Math.min(roi.width, Math.max(0.02, 1 - value));
      roi = { ...roi, x: roundRoiValue(Math.min(value, 1 - width)), width: roundRoiValue(width) };
    } else if (field === "y") {
      const height = Math.min(roi.height, Math.max(0.02, 1 - value));
      roi = { ...roi, y: roundRoiValue(Math.min(value, 1 - height)), height: roundRoiValue(height) };
    } else if (field === "width") {
      const width = Math.max(0.02, Math.min(1 - roi.x, value));
      roi = { ...roi, width: roundRoiValue(width) };
    } else {
      const height = Math.max(0.02, Math.min(1 - roi.y, value));
      roi = { ...roi, height: roundRoiValue(height) };
    }
  }

  function normalizedPointerCoords(e: PointerEvent): { nx: number; ny: number } | null {
    if (!roiContainerEl) return null;
    const rect = roiContainerEl.getBoundingClientRect();
    if (!rect.width || !rect.height) return null;
    return {
      nx: clamp01((e.clientX - rect.left) / rect.width),
      ny: clamp01((e.clientY - rect.top) / rect.height),
    };
  }

  function startRoiWindowListeners() {
    if (typeof window === "undefined") return;
    window.addEventListener("pointermove", handleRoiPointerMove);
    window.addEventListener("pointerup", handleRoiPointerUp);
    window.addEventListener("pointercancel", handleRoiPointerUp);
  }

  function stopRoiWindowListeners() {
    if (typeof window === "undefined") return;
    window.removeEventListener("pointermove", handleRoiPointerMove);
    window.removeEventListener("pointerup", handleRoiPointerUp);
    window.removeEventListener("pointercancel", handleRoiPointerUp);
  }

  function handleRoiResizeDown(e: PointerEvent, handle: RoiResizeHandle) {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    e.stopPropagation();
    const coords = normalizedPointerCoords(e);
    if (!coords) return;
    roiDragState = {
      mode: "resize",
      handle,
      startX: coords.nx,
      startY: coords.ny,
      originRoi: { ...roi },
    };
    startRoiWindowListeners();
  }

  function handleRoiPointerDown(e: PointerEvent) {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    const coords = normalizedPointerCoords(e);
    if (!coords) return;
    const inside =
      coords.nx >= roi.x &&
      coords.nx <= roi.x + roi.width &&
      coords.ny >= roi.y &&
      coords.ny <= roi.y + roi.height;
    roiDragState = {
      mode: inside ? "move" : "draw",
      startX: coords.nx,
      startY: coords.ny,
      originRoi: { ...roi },
    };
    startRoiWindowListeners();
  }

  function handleRoiPointerMove(e: PointerEvent) {
    if (!roiDragState) return;
    const coords = normalizedPointerCoords(e);
    if (!coords) return;
    if (roiDragState.mode === "move") {
      const dx = coords.nx - roiDragState.startX;
      const dy = coords.ny - roiDragState.startY;
      const x = Math.max(0, Math.min(1 - roiDragState.originRoi.width, roiDragState.originRoi.x + dx));
      const y = Math.max(0, Math.min(1 - roiDragState.originRoi.height, roiDragState.originRoi.y + dy));
      roi = { ...roiDragState.originRoi, x: roundRoiValue(x), y: roundRoiValue(y) };
    } else if (roiDragState.mode === "resize" && roiDragState.handle) {
      const orig = roiDragState.originRoi;
      let left = orig.x;
      let top = orig.y;
      let right = orig.x + orig.width;
      let bottom = orig.y + orig.height;
      const h = roiDragState.handle;
      if (h.includes("w")) left = clamp01(Math.min(coords.nx, right - MIN_ROI_SIZE));
      if (h.includes("e")) right = clamp01(Math.max(coords.nx, left + MIN_ROI_SIZE));
      if (h.includes("n")) top = clamp01(Math.min(coords.ny, bottom - MIN_ROI_SIZE));
      if (h.includes("s")) bottom = clamp01(Math.max(coords.ny, top + MIN_ROI_SIZE));
      roi = {
        x: roundRoiValue(left),
        y: roundRoiValue(top),
        width: roundRoiValue(right - left),
        height: roundRoiValue(bottom - top),
      };
    } else {
      const x = Math.min(roiDragState.startX, coords.nx);
      const y = Math.min(roiDragState.startY, coords.ny);
      const width = Math.max(MIN_ROI_SIZE, Math.min(1 - x, Math.abs(coords.nx - roiDragState.startX)));
      const height = Math.max(MIN_ROI_SIZE, Math.min(1 - y, Math.abs(coords.ny - roiDragState.startY)));
      roi = {
        x: roundRoiValue(x),
        y: roundRoiValue(y),
        width: roundRoiValue(width),
        height: roundRoiValue(height),
      };
    }
  }

  function handleRoiPointerUp() {
    roiDragState = null;
    stopRoiWindowListeners();
  }

  function formatTimestamp(ms: number): string {
    const totalSeconds = Math.max(0, Math.floor(ms / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    const pad = (n: number) => String(n).padStart(2, "0");
    return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
  }

  function hardcodedOutputName(mediaPath: string): string {
    const basename = mediaPath.split(/[\\/]/).pop() || "film";
    const stem = basename.replace(/\.[^.]+$/, "").replace(/[<>:"/\\|?*\x00-\x1f]/g, "_").replace(/\s+/g, "_").replace(/[. ]+$/, "") || "film";
    return `${stem}_hardcoded.srt`;
  }

  async function loadPreviewFrame(timeMs?: number | null) {
    if (disposed || !path || !isVideoMedia) return;
    const media = path;
    const requestedTime = timeMs === undefined ? frameTimeMs : timeMs;
    const request = frameRequests.begin();
    loadingFrame = true;
    try {
      const frame = await ocrVideoPreviewFrame(media, requestedTime);
      if (disposed || !request.isCurrent() || path !== media) return;
      framePreview = frame;
      if (frameTimeMs === null || timeMs !== undefined) {
        frameTimeMs = frame.time_ms;
      }
    } catch (e) {
      if (!disposed && request.isCurrent() && path === media) {
        snackbar.show(errorMessage(e), "error");
      }
    } finally {
      if (!disposed && request.isCurrent()) {
        loadingFrame = false;
      }
    }
  }

  function setExtractMode(mode: "embedded" | "hardcoded") {
    extractMode = mode;
    if (mode === "hardcoded" && isVideoMedia && framePreview === null && !loadingFrame) {
      void loadPreviewFrame();
    }
  }

  function seekPreviewFrame(deltaMs: number) {
    if (!framePreview || loadingFrame || !path) return;
    const base = frameTimeMs ?? framePreview.time_ms;
    const max = framePreview.duration_ms > 0 ? framePreview.duration_ms : base + deltaMs;
    const target = Math.max(0, Math.min(max, base + deltaMs));
    frameTimeMs = target;
    void loadPreviewFrame(target);
  }

  function effectiveHardcodedLanguage(): string | null {
    return hardcodedLanguage && hardcodedLanguage !== "auto" ? hardcodedLanguage : null;
  }

  async function previewHardcoded() {
    if (disposed || busy || extractingHardcoded || downloadingIndex !== null || !isVideoMedia || !canOcr || aiStore.isOcring || isPreviewOcrActive()) return;
    const tiers = requireOcrTiers();
    if (!tiers) return;
    await runOcrPreview("hardcoded", null, tiers, (media) =>
      previewOcrHardcodedSubtitle({
        path: media,
        roi: { ...roi },
        sampleFps,
        language: effectiveHardcodedLanguage(),
        tiers,
      }),
    );
  }

  async function extractHardcoded() {
    if (disposed || busy || extractingHardcoded || downloadingIndex !== null || !isVideoMedia || !canOcr || aiStore.isOcring) return;
    const tiers = requireOcrTiers();
    if (!tiers) return;
    error = "";
    extractingHardcoded = true;
    ocrProgress = null;
    ocrCancelling = false;
    activeOcrUsesCloud = tiersHaveCloud(tiers);
    ownsOcr = true;
    aiStore.isOcring = true;
    let detachListener: (() => void) | null = null;
    try {
      const directory = await resolveOutputDir();
      if (disposed) return;
      const outputPath = await join(directory, hardcodedOutputName(path));
      if (disposed) return;
      detachListener = await attachOcrProgressListener(path);
      if (disposed) return;
      if (ocrCancelling) {
        snackbar.show(t("extract.ocrCancelled"), "info");
        return;
      }
      await ocrHardcodedSubtitle({
        path,
        outputPath,
        roi: { ...roi },
        sampleFps,
        language: effectiveHardcodedLanguage(),
        tiers,
      });
      if (disposed) return;
      if (ocrCancelling) {
        snackbar.show(t("extract.ocrCancelled"), "info");
        return;
      }
      snackbar.show(`${t("extract.saved")} ${outputPath}`, "success");
    } catch (e) {
      reportOcrOrExtractError(e, true);
    } finally {
      detachListener?.();
      releaseOcrOwnership();
      if (!disposed) {
        extractingHardcoded = false;
        ocrProgress = null;
        ocrCancelling = false;
      }
    }
  }

</script>
<div class="h-full flex flex-col bg-gray-900 overflow-hidden">
  <div class="flex flex-1 flex-col overflow-hidden p-6 min-h-0 text-gray-100">
    <WorkflowWorkspace>
      {#snippet files()}
      <div class="glass-card p-5" class:ring-2={isDraggingOver} class:ring-teal-400={isDraggingOver}>
        <h3 class="mb-4 text-lg font-semibold panel-title-files-output flex items-center gap-2">
          <svg aria-hidden="true" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" /></svg>
          {t("common.filesAndOutput")}
        </h3>
        <div class="grid grid-cols-1 gap-4 md:grid-cols-2 md:items-end">
        <PathPickerField labelIcon="M15 10l5-3v10l-5-3M4 5h9a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z" label={t("flashcards.mediaFile")} value={path} placeholder={t("extract.choose")}
          browseTitle={t("flashcards.browse")} browseLabel={t("flashcards.browse")} onbrowse={chooseMedia}
          disabled={busy || downloadingIndex !== null || extractingHardcoded} onclear={() => { resetMediaState(); path = ""; error = ""; }} />
        <PathPickerField label={t("flashcards.outputDir")} labelIcon="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2Z" value={outputDir} placeholder={t("flashcards.outputDir")} browseTitle={t("flashcards.browse")} browseLabel={t("flashcards.browse")} onbrowse={chooseOutput} disabled={busy || downloadingIndex !== null || extractingHardcoded} />
        </div>
      </div>
      {/snippet}

      <div class="glass-card p-5">
        <div role="status" aria-live="polite">
        {#if error || busy || ((downloadingIndex !== null || extractingHardcoded) && (ocrProgress || ocrCancelling || aiStore.isOcring))}
          <div class="mb-3 flex flex-col gap-2 text-sm">
            {#if error}
              <div class="flex flex-wrap items-center justify-between gap-2 min-h-5">
                <span class="text-red-300">{error}</span>
                {#if onGoToSettings && error === t("extract.ocrNoProvider")}
                  <SettingsJumpButton label={t("settings.section.itt")} section="itt" onjump={onGoToSettings} variant="cyan" />
                {/if}
              </div>
            {:else if busy}
              <div class="flex min-h-5 items-center"><span class="inline-flex" aria-label={t("extract.working")}><LoadingSpinner /></span></div>
            {/if}
            {#if (downloadingIndex !== null || extractingHardcoded) && (ocrProgress || ocrCancelling || aiStore.isOcring)}
              <div class="relative flex items-center justify-between gap-3 overflow-hidden rounded-lg border border-cyan-400/20 bg-cyan-500/5 px-3 py-2">
                <ProgressFill value={ocrProgress?.percentage ?? 0} />
                <div class="relative z-10 flex min-w-0 items-center gap-2 text-xs text-cyan-200">
                  <span class="truncate font-medium">
                    {#if ocrProgress && ocrProgress.total > 0}
                      {t("extract.ocrInProgress", { current: ocrProgress.current, total: ocrProgress.total })}
                    {:else}
                      {t("extract.working")}
                    {/if}
                  </span>
                  {#if ocrProgress?.message}
                    <span class="truncate text-cyan-300/80">· {ocrProgress.message}</span>
                  {/if}
                </div>
                <button
                  type="button"
                  class="relative z-10 inline-flex shrink-0 items-center gap-1.5 rounded-md border border-red-400/35 bg-red-500/20 px-2.5 py-1 text-xs font-medium text-red-200 transition-colors hover:bg-red-500/30 disabled:cursor-not-allowed disabled:opacity-50 cursor-pointer"
                  disabled={ocrCancelling}
                  onclick={cancelOcr}
                >
                  <svg aria-hidden="true" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="6" width="12" height="12" rx="1.5" /></svg>
                  {ocrCancelling ? t("refine.btn.stopping") : t("extract.ocrCancel")}
                </button>
              </div>
            {/if}
          </div>
        {/if}
        </div>

        {#if extractMode === "embedded"}
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="relative w-full sm:w-80">
              <svg aria-hidden="true" class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4 4" /></svg>
            <input disabled={busy || tracks.length === 0} type="search" value={search} oninput={(event) => { search = event.currentTarget.value; page = 0; }} aria-label={t("extract.search")} placeholder={t("extract.search")} class="w-full rounded-lg border border-white/10 bg-black/30 pl-10 pr-4 py-2.5 text-sm outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-500/20 disabled:opacity-50" />
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <span class="mr-2 text-xs text-gray-400">{groups.length} · {filtered.length} / {tracks.length}</span>
              <button class="btn-secondary inline-flex items-center gap-1.5 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-30" disabled={busy || tracks.length === 0 || page === 0} onclick={() => page -= 1}>
                <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m15 6-6 6 6 6" /></svg>{t("extract.previous")}
              </button>
              <span class="min-w-10 text-center text-xs text-gray-400" aria-live="polite">{page + 1} / {pageCount}</span>
              <button class="btn-secondary inline-flex items-center gap-1.5 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-30" disabled={busy || tracks.length === 0 || page + 1 >= pageCount} onclick={() => page += 1}>
                {t("extract.next")}<svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 6 6 6-6 6" /></svg>
              </button>
            </div>
          </div>
          <div class="relative mt-4 grid min-h-[520px] grid-cols-1 lg:grid-cols-2 content-start gap-3" aria-busy={busy}>
            {#if !path && !busy}
              <EmptyStatusLabel message={t("media.noMediaLoaded")} />
            {/if}
            {#each visible as group (group.key)}
              <SubtitleTrackCard {group} busy={busy || extractingHardcoded} {downloaded} {downloadingIndex} selectedIndex={selections[group.key]} {canOcr} onselect={(index) => selections = { ...selections, [group.key]: index }} ondownload={extract} onpreview={preview} />
            {/each}
            {#if visible.length === 0}
              {#each Array(SUBTITLE_PAGE_SIZE) as _, index (index)}
                <div aria-hidden="true" class="flex min-h-[108px] items-center justify-between rounded-xl border border-white/5 bg-black/10 p-4 {busy ? 'animate-pulse motion-reduce:animate-none' : ''}">
                  <div class="space-y-2"><div class="h-3 w-36 rounded bg-white/5"></div><div class="h-2 w-24 rounded bg-white/[0.03]"></div></div>
                  <div class="h-10 w-10 rounded-lg border border-white/5"></div>
                </div>
              {/each}
            {/if}
            {#if tracksScanned && tracks.length === 0}
              <div class="pointer-events-none absolute inset-0 flex items-center justify-center p-4" role="status" aria-live="polite">
                <span class="rounded-xl border border-red-400/25 bg-gray-900/95 px-5 py-3 text-center text-sm font-semibold text-red-300 shadow-lg">{t("extract.empty")}</span>
              </div>
            {/if}
          </div>
          {#if tracks.length > 0 && filtered.length === 0}<p class="mt-4 text-sm text-gray-400" role="status">{t("extract.noMatches")}</p>{/if}
        {:else}
          <div id="hardcoded-roi-panel" class="space-y-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="min-w-0">
                <h3 class="text-base font-semibold text-white">{t("extract.hardcodedTitle")}</h3>
                <p class="mt-0.5 text-xs text-gray-400">{t("extract.hardcodedDesc")}</p>
              </div>
              {#if framePreview}
                <span class="rounded-lg border border-cyan-400/25 bg-cyan-500/10 px-2.5 py-1 font-mono text-[11px] text-cyan-200">
                  ~{Math.max(1, Math.round(roi.width * framePreview.width))} × {Math.max(1, Math.round(roi.height * framePreview.height))} px
                </span>
              {/if}
            </div>

            {#if !path}
              <div class="relative flex aspect-video max-h-[360px] w-full items-center justify-center rounded-xl border border-white/10 bg-black/30">
                <EmptyStatusLabel message={t("media.noMediaLoaded")} />
              </div>
            {:else if !isVideoMedia}
              <div class="flex min-h-[220px] items-center justify-center rounded-xl border border-amber-400/20 bg-amber-500/5 p-6 text-center text-sm text-amber-200">
                {t("extract.hardcodedVideoRequired")}
              </div>
            {:else}
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex flex-wrap items-center gap-2">
                  <button type="button" class="inline-flex items-center gap-1.5 rounded-lg border border-sky-400/30 bg-sky-500/15 px-3 py-1.5 text-xs font-semibold text-sky-200 transition-colors hover:bg-sky-500/25 cursor-pointer" onclick={() => setRoiPreset("bottom")}>
                    <svg aria-hidden="true" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 16h10"/></svg>
                    {t("extract.hardcodedPresetBottom")}
                  </button>
                  <button type="button" class="inline-flex items-center gap-1.5 rounded-lg border border-violet-400/30 bg-violet-500/15 px-3 py-1.5 text-xs font-semibold text-violet-200 transition-colors hover:bg-violet-500/25 cursor-pointer" onclick={() => setRoiPreset("top")}>
                    <svg aria-hidden="true" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 8h10"/></svg>
                    {t("extract.hardcodedPresetTop")}
                  </button>
                  <button type="button" class="inline-flex items-center gap-1.5 rounded-lg border border-amber-400/30 bg-amber-500/15 px-3 py-1.5 text-xs font-semibold text-amber-200 transition-colors hover:bg-amber-500/25 cursor-pointer" onclick={() => setRoiPreset("full")}>
                    <svg aria-hidden="true" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/></svg>
                    {t("extract.hardcodedPresetFull")}
                  </button>
                  <button type="button" class="inline-flex items-center gap-1.5 rounded-lg border border-cyan-400/30 bg-cyan-500/15 px-3 py-1.5 text-xs font-semibold text-cyan-200 transition-colors hover:bg-cyan-500/25 cursor-pointer" onclick={centerRoiHorizontally}>
                    <svg aria-hidden="true" class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v18M5 8l4 4-4 4M19 8l-4 4 4 4"/></svg>
                    {t("extract.hardcodedCenter")}
                  </button>
                  <button type="button" class="inline-flex items-center gap-1.5 rounded-lg border border-emerald-400/30 bg-emerald-500/15 px-3 py-1.5 text-xs font-semibold text-emerald-200 transition-colors hover:bg-emerald-500/25 disabled:opacity-50 cursor-pointer" disabled={loadingFrame || !path} onclick={() => void loadPreviewFrame(frameTimeMs ?? framePreview?.time_ms ?? null)}>
                    <svg aria-hidden="true" class="h-3.5 w-3.5 {loadingFrame ? 'animate-spin' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
                    {t("extract.hardcodedPreviewFrame")}
                  </button>
                </div>
                <div class="flex flex-wrap items-center gap-3">
                  <div class="flex items-center gap-2 text-xs text-gray-300">
                    <span>{t("extract.hardcodedLanguage")}</span>
                    <div class="w-44 compact-select">
                      <SearchableSelect
                        options={hardcodedLanguageOptions}
                        value={hardcodedLanguage}
                        onchange={(val) => { hardcodedLanguage = val; }}
                        compact={true}
                        portal={true}
                        ariaLabel={t("extract.hardcodedLanguage")}
                        noResultsText={t("common.noResults")}
                      />
                    </div>
                  </div>
                  <div class="flex items-center gap-2 text-xs text-gray-300">
                    <span>{t("extract.hardcodedFps")}</span>
                    <div class="w-28 compact-select">
                      <SearchableSelect
                        options={fpsOptions}
                        value={String(sampleFps)}
                        onchange={(val) => { sampleFps = Number(val); }}
                        compact={true}
                        portal={true}
                        searchable={false}
                        ariaLabel={t("extract.hardcodedFps")}
                      />
                    </div>
                  </div>
                </div>
              </div>

              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                bind:this={roiContainerEl}
                class="relative mx-auto overflow-hidden rounded-xl border border-white/15 bg-black/50 select-none touch-none cursor-crosshair {framePreview?.data_url ? 'w-fit max-w-3xl' : 'w-full max-w-3xl'}"
                onpointerdown={handleRoiPointerDown}
              >
                {#if framePreview?.data_url}
                  <img
                    src={framePreview.data_url}
                    alt={t("extract.hardcodedTitle")}
                    class="block max-h-[380px] w-auto max-w-full pointer-events-none select-none"
                    draggable="false"
                  />
                {:else}
                  <div class="flex aspect-video max-h-[380px] w-full items-center justify-center text-xs text-gray-400">
                    {#if loadingFrame}
                      <LoadingSpinner />
                    {:else}
                      {t("extract.hardcodedDesc")}
                    {/if}
                  </div>
                {/if}
                <div
                  data-testid="roi-overlay"
                  class="pointer-events-none absolute border-2 border-cyan-400 bg-cyan-400/15 shadow-[0_0_0_9999px_rgba(0,0,0,0.45)]"
                  style:left={`${(roi.x * 100).toFixed(2)}%`}
                  style:top={`${(roi.y * 100).toFixed(2)}%`}
                  style:width={`${(roi.width * 100).toFixed(2)}%`}
                  style:height={`${(roi.height * 100).toFixed(2)}%`}
                >
                  {#each ROI_HANDLES as item (item.handle)}
                    <button
                      type="button"
                      aria-label={`${t("extract.hardcodedTitle")} (${item.handle.toUpperCase()})`}
                      class="pointer-events-auto absolute border-gray-950 bg-cyan-300 {item.className}"
                      onpointerdown={(e) => handleRoiResizeDown(e, item.handle)}
                    ></button>
                  {/each}
                </div>
              </div>

              {#if framePreview && framePreview.duration_ms > 0}
                <div class="mx-auto flex w-full max-w-3xl items-center gap-2.5">
                  <button type="button" class="inline-flex items-center gap-1 rounded-lg border border-emerald-400/25 bg-emerald-500/10 px-2 py-1 text-[11px] font-semibold text-emerald-200 hover:bg-emerald-500/20 disabled:opacity-40 cursor-pointer" disabled={loadingFrame} onclick={() => seekPreviewFrame(-5000)} title="-5s">
                    <svg aria-hidden="true" class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m11 17-5-5 5-5M18 17l-5-5 5-5"/></svg>
                    5s
                  </button>
                  <span class="w-14 shrink-0 text-right font-mono text-xs text-gray-400">{formatTimestamp(frameTimeMs ?? framePreview.time_ms)}</span>
                  <input
                    type="range"
                    aria-label={t("extract.hardcodedPreviewFrame")}
                    min="0"
                    max={framePreview.duration_ms}
                    step="500"
                    value={frameTimeMs ?? framePreview.time_ms}
                    oninput={(e) => { frameTimeMs = Number(e.currentTarget.value); }}
                    onchange={() => void loadPreviewFrame(frameTimeMs)}
                    class="h-1.5 flex-1 cursor-pointer accent-cyan-400"
                  />
                  <span class="w-14 shrink-0 font-mono text-xs text-gray-500">{formatTimestamp(framePreview.duration_ms)}</span>
                  <button type="button" class="inline-flex items-center gap-1 rounded-lg border border-emerald-400/25 bg-emerald-500/10 px-2 py-1 text-[11px] font-semibold text-emerald-200 hover:bg-emerald-500/20 disabled:opacity-40 cursor-pointer" disabled={loadingFrame} onclick={() => seekPreviewFrame(5000)} title="+5s">
                    5s
                    <svg aria-hidden="true" class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m6 17 5-5-5-5M13 17l5-5-5-5"/></svg>
                  </button>
                </div>
              {/if}

              <div class="mx-auto grid w-full max-w-3xl grid-cols-2 gap-3 sm:grid-cols-4">
                <label class="flex flex-col gap-1 text-[11px] text-gray-400">
                  <span>X (%)</span>
                  <input type="number" min="0" max="98" step="1" aria-label="X (%)" value={Math.round(roi.x * 100)} oninput={(e) => updateRoiPercent("x", Number(e.currentTarget.value))} class="rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5 text-xs text-white outline-none focus:border-cyan-400" />
                </label>
                <label class="flex flex-col gap-1 text-[11px] text-gray-400">
                  <span>Y (%)</span>
                  <input type="number" min="0" max="98" step="1" aria-label="Y (%)" value={Math.round(roi.y * 100)} oninput={(e) => updateRoiPercent("y", Number(e.currentTarget.value))} class="rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5 text-xs text-white outline-none focus:border-cyan-400" />
                </label>
                <label class="flex flex-col gap-1 text-[11px] text-gray-400">
                  <span>W (%)</span>
                  <input type="number" min="2" max="100" step="1" aria-label="W (%)" value={Math.round(roi.width * 100)} oninput={(e) => updateRoiPercent("width", Number(e.currentTarget.value))} class="rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5 text-xs text-white outline-none focus:border-cyan-400" />
                </label>
                <label class="flex flex-col gap-1 text-[11px] text-gray-400">
                  <span>H (%)</span>
                  <input type="number" min="2" max="100" step="1" aria-label="H (%)" value={Math.round(roi.height * 100)} oninput={(e) => updateRoiPercent("height", Number(e.currentTarget.value))} class="rounded-lg border border-white/10 bg-black/30 px-2.5 py-1.5 text-xs text-white outline-none focus:border-cyan-400" />
                </label>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </WorkflowWorkspace>
  </div>

  <FooterActions>
    {#snippet left()}
      <div class="relative grid grid-cols-2 rounded-xl border border-white/10 bg-black/30 p-1 select-none" role="tablist" aria-label={t("nav.extract")}>
        <span
          aria-hidden="true"
          class="absolute bottom-1 top-1 left-1 w-[calc(50%-4px)] rounded-lg border transition-transform duration-200 ease-out {extractMode === 'embedded' ? 'translate-x-0 border-teal-400/40 bg-teal-500/20' : 'translate-x-[calc(100%+4px)] border-cyan-400/40 bg-cyan-500/20'}"
        ></span>
        <button
          type="button"
          role="tab"
          aria-selected={extractMode === "embedded"}
          onclick={() => setExtractMode("embedded")}
          class="relative z-10 inline-flex items-center justify-center gap-2 px-4 py-2 text-xs font-semibold transition-colors cursor-pointer {extractMode === 'embedded' ? 'text-teal-100' : 'text-gray-400 hover:text-gray-200'}"
        >
          <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 13h4m3 0h3M7 16h10"/></svg>
          {t("extract.modeEmbedded")}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={extractMode === "hardcoded"}
          onclick={() => setExtractMode("hardcoded")}
          class="relative z-10 inline-flex items-center justify-center gap-2 px-4 py-2 text-xs font-semibold transition-colors cursor-pointer {extractMode === 'hardcoded' ? 'text-cyan-100' : 'text-gray-400 hover:text-gray-200'}"
        >
          <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 8V5a1 1 0 011-1h3M16 4h3a1 1 0 011 1v3M4 16v3a1 1 0 001 1h3m9 0h3a1 1 0 001-1v-3"/><path d="M8 14h8M9 10h6"/></svg>
          {t("extract.modeHardcoded")}
        </button>
      </div>
    {/snippet}

    {#snippet right()}
      {#if (downloadingIndex !== null || extractingHardcoded) && (ocrProgress || ocrCancelling || aiStore.isOcring)}
        <div class="flex items-center gap-3">
          {#if ocrProgress && ocrProgress.total > 0}
            <span class="text-xs font-medium text-cyan-200">{t("extract.ocrInProgress", { current: ocrProgress.current, total: ocrProgress.total })}</span>
          {:else}
            <span class="text-xs font-medium text-cyan-200">{t("extract.working")}</span>
          {/if}
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-xl border border-red-400/35 bg-red-500/20 px-4 py-2.5 text-xs font-semibold text-red-200 transition-colors hover:bg-red-500/30 disabled:cursor-not-allowed disabled:opacity-50 cursor-pointer"
            disabled={ocrCancelling}
            onclick={cancelOcr}
          >
            <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="6" y="6" width="12" height="12" rx="1.5" /></svg>
            {ocrCancelling ? t("refine.btn.stopping") : t("extract.ocrCancel")}
          </button>
        </div>
      {:else if extractMode === "hardcoded"}
        <div class="flex flex-wrap items-center gap-2.5">
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded-xl border border-indigo-400/35 bg-indigo-500/20 px-4 py-2.5 text-xs font-semibold text-indigo-200 transition-colors hover:bg-indigo-500/30 disabled:cursor-not-allowed disabled:opacity-40 cursor-pointer"
            disabled={busy || extractingHardcoded || downloadingIndex !== null || !isVideoMedia || !canOcr || aiStore.isOcring}
            onclick={() => void previewHardcoded()}
          >
            <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6Z"/><circle cx="12" cy="12" r="2.5"/></svg>
            {t("extract.hardcodedPreviewOcr")}
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded-xl border border-teal-400/40 bg-teal-500/25 px-4 py-2.5 text-xs font-semibold text-teal-100 shadow-lg shadow-teal-500/15 transition-colors hover:bg-teal-500/35 disabled:cursor-not-allowed disabled:opacity-40 cursor-pointer"
            disabled={busy || extractingHardcoded || downloadingIndex !== null || !isVideoMedia || !canOcr || aiStore.isOcring}
            onclick={() => void extractHardcoded()}
          >
            <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 4v12m0 0-4-4m4 4 4-4M4 17v2a1 1 0 001 1h14a1 1 0 001-1v-2"/></svg>
            {t("extract.hardcodedSave")}
          </button>
        </div>
      {:else}
        <div class="flex items-center gap-3">
          {#if !canOcr && onGoToSettings}
            <SettingsJumpButton label={t("settings.section.itt")} section="itt" onjump={onGoToSettings} variant="amber" />
          {/if}
          {#if tracksScanned && tracks.length === 0 && isVideoMedia}
            <button
              type="button"
              onclick={() => setExtractMode("hardcoded")}
              class="inline-flex items-center gap-2 rounded-xl border border-cyan-400/35 bg-cyan-500/20 px-4 py-2.5 text-xs font-semibold text-cyan-200 transition-colors hover:bg-cyan-500/30 cursor-pointer"
            >
              <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 8V5a1 1 0 011-1h3M16 4h3a1 1 0 011 1v3M4 16v3a1 1 0 001 1h3m9 0h3a1 1 0 001-1v-3"/><path d="M8 14h8M9 10h6"/></svg>
              {t("extract.modeHardcoded")}
            </button>
          {/if}
        </div>
      {/if}
    {/snippet}
  </FooterActions>
</div>

{#if previewOpen}
  <SubtitlePreviewModal
    title={previewMode === "hardcoded" ? `${t("extract.preview")} · ${t("extract.hardcodedTitle")}` : `${t("extract.preview")} · #${previewTrack!.index} · ${previewTrack!.title || previewTrack!.language}`}
    text={previewText}
    error={previewError}
    loading={previewLoading}
    saving={downloadingIndex !== null || extractingHardcoded}
    hasPrevious={previewMode === "track" && previewPosition > 0}
    hasNext={previewMode === "track" && previewPosition + 1 < previewVariants.length}
    onprevious={() => { if (previewMode === "track") void preview(previewVariants[previewPosition - 1]); }}
    onnext={() => { if (previewMode === "track") void preview(previewVariants[previewPosition + 1]); }}
    onselect={selectPreview}
    ondownload={() => { if (previewMode === "hardcoded") void extractHardcoded(); else if (previewTrack) void extract(previewTrack); }}
    onclose={closePreview}
  />
{/if}

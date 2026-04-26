<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "./dialogGuard";
  import PathPreviewModal from "./PathPreviewModal.svelte";
  import Snackbar from "./Snackbar.svelte";
  import InfoModal from "./InfoModal.svelte";
  import InfoButton from "./InfoButton.svelte";
  import { syncSections } from "./info";
  import { onMount } from "svelte";
  import { locale } from "./i18n";

  interface Props {
    active?: boolean;
  }

  let { active = true }: Props = $props();

  let t = $derived($locale);

  interface SubtitleInfo {
    id: number;
    start_ms: number;
    end_ms: number;
    text: string;
    synced_start_ms: number;
    synced_end_ms: number;
    offset_ms: number;
    is_anchor: boolean;
  }

  interface SyncStatus {
    is_loaded: boolean;
    srt_path: string | null;
    video_path: string | null;
    total_subtitles: number;
    anchor_count: number;
    checked_count: number;
    completion_percentage: number;
    average_offset_ms: number;
    suggested_next_id: number | null;
  }

  interface AnchorInfo {
    subtitle_id: number;
    original_time_ms: number;
    corrected_time_ms: number;
    offset_ms: number;
    is_manual: boolean;
  }

  let audioElement = $state<HTMLMediaElement | null>(null);
  let status = $state<SyncStatus | null>(null);
  let subtitles = $state<SubtitleInfo[]>([]);
  let anchors = $state<AnchorInfo[]>([]);
  let currentVideoTime = $state(0);
  let isPlaying = $state(false);
  let error = $state<string | null>(null);
  let audioSrc = $state<string | null>(null);
  let audioError = $state<string | null>(null);
  let hasAutoPaused = $state(false);

  let showResetModal = $state(false);

  let snackbarMessage = $state<string | null>(null);
  let snackbarVariant = $state<"success" | "info" | "warning" | "error">("info");
  let snackbarTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  let confidenceScore = $derived(
    status?.total_subtitles
      ? Math.min(100, Math.round((anchors.length / Math.max(1, status.total_subtitles / 15)) * 100))
      : 0
  );

  let wizardSubtitle = $state<SubtitleInfo | null>(null);
  let offsetAdjustment = $state(0);
  let targetOffset = 0;
  let offsetUpdateFrame = 0;

  function updateOffset(delta: number) {
    targetOffset += delta;
    if (offsetUpdateFrame) cancelAnimationFrame(offsetUpdateFrame);
    offsetUpdateFrame = requestAnimationFrame(() => {
      offsetAdjustment = targetOffset;
    });
  }

  function resetOffset() {
    targetOffset = 0;
    offsetAdjustment = 0;
  }

  let wizardHistory = $state<number[]>([]);
  let showSaveSuggestion = $state(false);
  let manualGoToId = $state("");

  const PAGE_SIZE = 50;
  let currentPage = $state(1);
  let totalPages = $derived(status?.total_subtitles ? Math.ceil(status.total_subtitles / PAGE_SIZE) : 0);
  let subtitleListElement = $state<HTMLDivElement | null>(null);

  let isDraggingOver = $state(false);
  let audioDuration = $state(0);
  let hasAudio = $derived(!!audioSrc && !audioError);

  let subtitleContextMenu = $state<{
    x: number;
    y: number;
    sub: SubtitleInfo;
  } | null>(null);

  let isNavigating = $state(false);
  let isStartingPlayback = $state(false);
  let isConfirmingCheckpoint = $state(false);

  let helpSection = $state<string | null>(null);
  let expandedPathField = $state<"srt" | "media" | null>(null);

  interface SyncLogEntry {
    id: number;
    timestamp: string;
    message: string;
    level: "info" | "success" | "warning" | "error";
  }
  let syncLogId = 0;
  let syncLogs = $state<SyncLogEntry[]>([]);

  let manualAnchors = $derived(anchors.filter((a) => a.is_manual));

  function addSyncLog(
    message: string,
    level: SyncLogEntry["level"] = "info",
  ) {
    const timestamp = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    syncLogs = [
      ...syncLogs,
      { id: ++syncLogId, timestamp, message, level },
    ].slice(-80);
  }

  function clearSyncLogs() {
    syncLogs = [];
    syncLogId = 0;
  }

  // ─── Auto-Sync with Whisper ────────────────────────────
  interface AutoSyncProgressPayload {
    stage: string;
    message: string;
    percentage: number;
    message_key?: string | null;
    params?: Record<string, string> | null;
  }

  let isAutoSyncing = $state(false);
  let isCancellingAutoSync = $state(false);
  let autoSyncProgress = $state(0);
  let autoSyncMessage = $state("");

  function resolveAutoSyncProgressMessage(payload: AutoSyncProgressPayload): string {
    if (payload.message_key) {
      const params = payload.params ?? {};

      // Keep compatibility with locale strings that expect {{count}} while
      // backend progress events provide { total } for segment counts.
      if (params.total && !params.count) {
        return t(payload.message_key, { ...params, count: params.total });
      }

      return t(payload.message_key, params);
    }
    return payload.message;
  }

  async function cancelAutoSync() {
    if (!isAutoSyncing || isCancellingAutoSync) return;
    isCancellingAutoSync = true;
    try {
      await invoke("sync_cancel_auto_sync");
      autoSyncMessage = t("sync.autoSyncCancelling");
    } catch (e) {
      showSnackbar(`Cancel failed: ${e}`, "error");
      isCancellingAutoSync = false;
    }
  }
  let whisperModelsAvailable = $state<string[]>([]);

  async function checkWhisperModels() {
    try {
      const models = await invoke<Array<{ id: string; downloaded: boolean }>>(
        "transcribe_list_models",
      );
      whisperModelsAvailable = models
        .filter((m) => m.downloaded)
        .map((m) => m.id);
    } catch {
      whisperModelsAvailable = [];
    }
  }

  async function startAutoSync() {
    if (isAutoSyncing) return;
    if (!status?.is_loaded) {
      showSnackbar(t("sync.dropSrtFirst"), "warning");
      return;
    }
    if (!status?.video_path && !audioSrc) {
      showSnackbar(t("sync.needVideoForAnchor"), "warning");
      return;
    }

    await checkWhisperModels();
    if (whisperModelsAvailable.length === 0) {
      showSnackbar(
        t("transcribe.noBackendWarning"),
        "warning"
      );
      return;
    }

    // Prefer small > base > tiny > medium > large
    const preferredOrder = ["small", "base", "tiny", "medium", "large"];
    const modelId =
      preferredOrder.find((m) => whisperModelsAvailable.includes(m)) ??
      whisperModelsAvailable[0];

    isAutoSyncing = true;
    autoSyncProgress = 0;
    autoSyncMessage = t("sync.autoSyncInProgress");
    addSyncLog(`Auto-sync started with model: ${modelId}`, "info");

    // Listen for progress events
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<AutoSyncProgressPayload>("sync-auto-progress", (event) => {
      autoSyncProgress = event.payload.percentage;
      autoSyncMessage = resolveAutoSyncProgressMessage(event.payload);
    });

    try {
      const result = await invoke<{
        success: boolean;
        cancelled: boolean;
        anchors_created: number;
        segments_analyzed: number;
        message: string;
      }>("sync_auto_sync", {
        modelId,
        language: null,
      });

      status = await invoke<SyncStatus>("sync_get_status");
      await refreshCurrentSubtitles();
      await loadAnchors();

      let summaryMessage = "";
      let summaryLevel: SyncLogEntry["level"] = "warning";

      if (result.cancelled) {
        summaryMessage = t("sync.autoSyncResult.cancelled");
      } else if (result.success) {
        summaryMessage = t("sync.autoSyncResult.success", {
          count: result.anchors_created,
        });
        summaryLevel = "success";
      } else {
        summaryMessage = t("sync.autoSyncResult.noMatches");
      }

      showSnackbar(summaryMessage, summaryLevel);
      addSyncLog(summaryMessage, summaryLevel);
    } catch (e) {
      const msg = `Auto-sync failed: ${e}`;
      showSnackbar(msg, "error");
      addSyncLog(msg, "error");
    } finally {
      isAutoSyncing = false;
      isCancellingAutoSync = false;
      autoSyncProgress = 0;
      autoSyncMessage = "";
      unlisten();
    }
  }

  type SyncPanelId = "toolbar" | "wizard" | "status" | "subtitleList";

  function syncDebug(message: string, payload?: Record<string, unknown>) {
    if (payload) {
      console.info(`[SyncTab] ${message}`, payload);
      return;
    }
    console.info(`[SyncTab] ${message}`);
  }

  function getFileName(path: string): string {
    return path.split("/").pop() || path;
  }

  async function tryAutoSelectMediaForSrt(srtPath: string) {
    try {
      const suggestedPath = await invoke<string | null>("sync_suggest_media_for_srt", {
        srtPath,
      });
      if (!suggestedPath) return;

      audioError = null;
      cleanupAudioSrc();
      audioSrc = await loadMediaFile(suggestedPath);
      status = await invoke<SyncStatus>("sync_set_video", { path: suggestedPath });
      showSnackbar(`Auto-selected media: ${getFileName(suggestedPath)}`, "success");
      addSyncLog(`Auto-selected media: ${getFileName(suggestedPath)}`, "success");
    } catch (e) {
      syncDebug("auto-media-suggestion-failed", { error: String(e) });
      addSyncLog("Auto media match not found", "warning");
    }
  }

  async function safePlayAudio(source: string): Promise<boolean> {
    if (!audioElement || !hasAudio) {
      syncDebug(`${source}: play skipped`, {
        hasAudio,
        hasElement: !!audioElement,
        audioError,
      });
      return false;
    }
    if (isStartingPlayback) {
      syncDebug(`${source}: play ignored (already starting)`);
      return false;
    }

    isStartingPlayback = true;
    try {
      syncDebug(`${source}: play requested`, {
        currentTime: audioElement.currentTime,
        paused: audioElement.paused,
        readyState: audioElement.readyState,
      });
      await audioElement.play();
      syncDebug(`${source}: play started`, {
        currentTime: audioElement.currentTime,
        paused: audioElement.paused,
      });
      return true;
    } catch (e) {
      const err = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      const isBenignAbort =
        err.includes("AbortError") ||
        err.toLowerCase().includes("operation was aborted");
      syncDebug(`${source}: play failed`, { error: err });
      if (!isBenignAbort) {
        showSnackbar(`Play error: ${err}`, "error");
      }
      return false;
    } finally {
      isStartingPlayback = false;
    }
  }

  function openSubtitleContextMenu(e: MouseEvent, sub: SubtitleInfo) {
    e.preventDefault();
    subtitleContextMenu = { x: e.clientX, y: e.clientY, sub };
  }

  function closeSubtitleContextMenu() {
    subtitleContextMenu = null;
  }

  async function playSubtitleFromList(sub: SubtitleInfo) {
    if (isNavigating) return;
    isNavigating = true;
    try {
      syncDebug("playSubtitleFromList", { subtitleId: sub.id });
      wizardSubtitle = sub;
      resetOffset();
      showSaveSuggestion = false;
      if (!wizardHistory.includes(sub.id)) {
        wizardHistory = [...wizardHistory, sub.id];
      }
      seekToSubtitleStart(sub);
      scrollToSubtitle(sub.id);
      await safePlayAudio("playSubtitleFromList");
    } finally {
      isNavigating = false;
    }
  }

  let mediaServerPort: number | null = null;

  async function getMediaPort(): Promise<number> {
    if (mediaServerPort) return mediaServerPort;
    mediaServerPort = await invoke<number>("get_media_server_port");
    return mediaServerPort;
  }

  async function loadMediaFile(filePath: string): Promise<string> {
    const port = await getMediaPort();
    return `http://127.0.0.1:${port}/media?path=${encodeURIComponent(filePath)}`;
  }

  function cleanupAudioSrc() {
    audioSrc = null;
  }

  function formatTime(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    const millis = Math.floor(ms % 1000);
    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
    }
    return `${minutes}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
  }

  function formatOffset(ms: number): string {
    const sign = ms >= 0 ? "+" : "";
    return `${sign}${(ms / 1000).toFixed(2)}s`;
  }

  function showSnackbar(message: string, variant: "success" | "info" | "warning" | "error" = "info") {
    if (snackbarTimeout) clearTimeout(snackbarTimeout);
    snackbarMessage = message;
    snackbarVariant = variant;
    snackbarTimeout = setTimeout(() => {
      snackbarMessage = null;
    }, 3500);
  }

  const OFFSET_TOLERANCE_MS = 200;

  function computeNextCheckpoint(): number | null {
    if (!status || status.total_subtitles === 0) return null;
    const total = status.total_subtitles;

    const initialCheckpoints = [
      1,
      Math.max(2, Math.round(total * 0.33)),
      Math.max(3, Math.round(total * 0.66)),
    ];

    for (const cp of initialCheckpoints) {
      if (!wizardHistory.includes(cp)) return cp;
    }

    if (areOffsetsConsistent()) return null;

    const sortedAnchors = [...manualAnchors].sort(
      (a, b) => a.subtitle_id - b.subtitle_id,
    );
    if (sortedAnchors.length < 2) {
      const mid = Math.round(total / 2);
      return wizardHistory.includes(mid) ? null : mid;
    }

    let maxDiff = 0;
    let bestMid = -1;
    for (let i = 0; i < sortedAnchors.length - 1; i++) {
      const a = sortedAnchors[i];
      const b = sortedAnchors[i + 1];
      const diff = Math.abs(a.offset_ms - b.offset_ms);
      const gap = b.subtitle_id - a.subtitle_id;
      if (diff > OFFSET_TOLERANCE_MS && gap > 1) {
        const mid = Math.round((a.subtitle_id + b.subtitle_id) / 2);
        if (!wizardHistory.includes(mid) && diff > maxDiff) {
          maxDiff = diff;
          bestMid = mid;
        }
      }
    }

    if (bestMid > 0) return bestMid;

    const first = sortedAnchors[0];
    const last = sortedAnchors[sortedAnchors.length - 1];
    if (first.subtitle_id > 2) {
      const mid = Math.round(first.subtitle_id / 2);
      if (!wizardHistory.includes(mid)) return mid;
    }
    if (last.subtitle_id < total - 1) {
      const mid = Math.round((last.subtitle_id + total) / 2);
      if (!wizardHistory.includes(mid)) return mid;
    }

    return null;
  }

  function areOffsetsConsistent(): boolean {
    if (manualAnchors.length < 2) return false;
    const offsets = manualAnchors.map((a) => a.offset_ms);
    const minOff = Math.min(...offsets);
    const maxOff = Math.max(...offsets);
    return maxOff - minOff <= OFFSET_TOLERANCE_MS;
  }

  async function skipCheckpoint() {
    if (wizardSubtitle) {
      if (!wizardHistory.includes(wizardSubtitle.id)) {
        wizardHistory = [...wizardHistory, wizardSubtitle.id];
      }
    }
    await advanceWizard();
  }

  async function advanceWizard() {
    console.debug("[SyncTab] advanceWizard called");
    const nextId = computeNextCheckpoint();
    if (nextId === null) {
      showSaveSuggestion = true;
      wizardSubtitle = null;
      return;
    }
    showSaveSuggestion = false;
    await goToCheckpoint(nextId);
  }

  async function goToCheckpoint(id: number) {
    console.debug(`[SyncTab] goToCheckpoint(${id}) called, isNavigating=${isNavigating}`);
    if (isNavigating) return;
    isNavigating = true;
    try {
      const sub = await invoke<SubtitleInfo>("sync_get_subtitle", { id });
      wizardSubtitle = sub;
      resetOffset();
      if (!wizardHistory.includes(id)) {
        wizardHistory = [...wizardHistory, id];
      }
      seekToSubtitleStart(sub);
      await loadSubtitlesAround(id);
      setTimeout(() => scrollToSubtitle(id), 50);
      void safePlayAudio("goToCheckpoint");
    } catch (e) {
      error = `Error loading subtitle: ${e}`;
    } finally {
      isNavigating = false;
    }
  }

  function seekToSubtitleStart(sub: SubtitleInfo) {
    if (!audioElement) return;
    const startSec = (sub.synced_start_ms + offsetAdjustment) / 1000;
    audioElement.currentTime = Math.max(0, startSec);
    hasAutoPaused = false;
  }

  function replayCurrentSubtitle() {
    if (!wizardSubtitle || !audioElement) return;
    seekToSubtitleStart(wizardSubtitle);
    void safePlayAudio("replayCurrentSubtitle");
  }

  async function selectSrtFile() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [{ name: "SRT Files", extensions: ["srt"] }],
      });
      if (selected) {
        const selectedPath = selected as string;
        status = await invoke<SyncStatus>("sync_load_srt", {
          path: selectedPath,
        });
        addSyncLog(`Loaded SRT: ${getFileName(selectedPath)}`, "success");
        await loadSubtitles();
        await loadAnchors();
        wizardHistory = [];
        showSaveSuggestion = false;
        await advanceWizard();
        await tryAutoSelectMediaForSrt(selectedPath);
      }
    } catch (e) {
      error = `${t("sync.errorLoadingSrt")} ${e}`;
      addSyncLog(`SRT load failed: ${e}`, "error");
    }
  }

  async function selectAudioFile() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: "Audio/Video Files",
            extensions: [
              "mp3",
              "wav",
              "ogg",
              "flac",
              "m4a",
              "aac",
              "wma",
              "opus",
              "m4b",
              "mp4",
              "mkv",
              "avi",
              "webm",
              "mov",
              "m4v",
              "m2ts",
              "mpeg",
              "mpg",
            ],
          },
        ],
      });
      if (selected) {
        const path = selected as string;
        audioError = null;
        cleanupAudioSrc();
        audioSrc = await loadMediaFile(path);
        status = await invoke<SyncStatus>("sync_set_video", { path });
        addSyncLog(`Loaded media: ${getFileName(path)}`, "success");
      }
    } catch (e) {
      error = `${t("sync.errorLoadingAudio")} ${e}`;
      addSyncLog(`Media load failed: ${e}`, "error");
    }
  }

  async function loadPage(page: number) {
    if (!status || status.total_subtitles === 0) return;
    if (page < 1) page = 1;
    if (page > totalPages) page = totalPages;
    currentPage = page;
    const startId = (page - 1) * PAGE_SIZE + 1;
    try {
      subtitles = await invoke<SubtitleInfo[]>("sync_get_subtitles_range", {
        startId,
        count: PAGE_SIZE,
      });
      if (subtitleListElement) {
        subtitleListElement.scrollTop = 0;
      }
    } catch (e) {
      error = `${t("sync.errorLoadingSrt")} ${e}`;
    }
  }

  async function loadSubtitles() {
    console.debug("[SyncTab] loadSubtitles");
    await loadPage(1);
  }

  async function loadSubtitlesAround(targetId: number) {
    console.debug(`[SyncTab] loadSubtitlesAround(${targetId})`);
    const page = Math.ceil(targetId / PAGE_SIZE);
    await loadPage(page);
  }

  function scrollToSubtitle(subtitleId: number) {
    if (!subtitleListElement) return;
    const element = subtitleListElement.querySelector(
      `[data-subtitle-id="${subtitleId}"]`,
    );
    if (element)
      element.scrollIntoView({ behavior: "smooth", block: "center" });
  }

  async function loadAnchors() {
    console.debug("[SyncTab] loadAnchors");
    try {
      anchors = await invoke<AnchorInfo[]>("sync_get_anchors");
    } catch (e) {
      error = `${t("sync.errorAddingAnchor")} ${e}`;
    }
  }

  async function refreshCurrentSubtitles() {
    console.debug(`[SyncTab] refreshCurrentSubtitles (page ${currentPage})`);
    await loadPage(currentPage);
  }

  async function confirmCurrentCheckpoint() {
    console.debug("[SyncTab] confirmCurrentCheckpoint called");
    if (isConfirmingCheckpoint) {
      syncDebug("confirmCurrentCheckpoint ignored (already running)");
      return;
    }
    if (!wizardSubtitle) return;
    if (!audioSrc || audioError) {
      showSnackbar(t("sync.needAudioForAnchor"), "warning");
      return;
    }

    const correctedTime = wizardSubtitle.synced_start_ms + offsetAdjustment;
    isConfirmingCheckpoint = true;

    try {
      syncDebug("confirmCurrentCheckpoint start", {
        subtitleId: wizardSubtitle.id,
        correctedTime: Math.round(correctedTime),
        offsetAdjustment,
      });
      status = await invoke<SyncStatus>("sync_add_anchor", {
        subtitleId: wizardSubtitle.id,
        correctedTimeMs: Math.round(correctedTime),
      });
      await refreshCurrentSubtitles();
      await loadAnchors();

      const updated = await invoke<SubtitleInfo>("sync_get_subtitle", {
        id: wizardSubtitle.id,
      });
      wizardSubtitle = updated;

      resetOffset();
      await advanceWizard();
      syncDebug("confirmCurrentCheckpoint completed", {
        subtitleId: updated.id,
      });
      addSyncLog(
        `Anchor confirmed on #${updated.id} (${formatOffset(updated.offset_ms)})`,
        "success",
      );
    } catch (e) {
      error = `${t("sync.errorAddingAnchor")} ${e}`;
      syncDebug("confirmCurrentCheckpoint failed", { error: String(e) });
      addSyncLog(`Anchor confirm failed: ${e}`, "error");
    } finally {
      isConfirmingCheckpoint = false;
    }
  }

  async function removeAnchor(subtitleId: number) {
    console.debug(`[SyncTab] removeAnchor(${subtitleId})`);
    try {
      status = await invoke<SyncStatus>("sync_remove_anchor", { subtitleId });
      await refreshCurrentSubtitles();
      await loadAnchors();
      addSyncLog(`Removed anchor #${subtitleId}`, "warning");
    } catch (e) {
      error = `${t("sync.errorRemovingAnchor")} ${e}`;
      addSyncLog(`Remove anchor failed: ${e}`, "error");
    }
  }

  async function goToSubtitleManual(sub: SubtitleInfo) {
    if (isNavigating) return;
    isNavigating = true;
    try {
      wizardSubtitle = sub;
      resetOffset();
      showSaveSuggestion = false;
      if (!wizardHistory.includes(sub.id)) {
        wizardHistory = [...wizardHistory, sub.id];
      }
      seekToSubtitleStart(sub);
      scrollToSubtitle(sub.id);
    } finally {
      isNavigating = false;
    }
  }

  async function goToLineById() {
    const id = parseInt(manualGoToId);
    if (isNaN(id) || id < 1) return;
    try {
      const sub = await invoke<SubtitleInfo>("sync_get_subtitle", { id });
      await goToSubtitleManual(sub);
      await loadSubtitlesAround(id);
      setTimeout(() => scrollToSubtitle(id), 50);
      manualGoToId = "";
    } catch (e) {
      showSnackbar(`Subtitle #${id} not found`, "warning");
    }
  }

  async function saveFile() {
    console.debug("[SyncTab] saveFile");
    try {
      const selected = await guardedSave({
        filters: [{ name: "SRT Files", extensions: ["srt"] }],
        defaultPath: status?.srt_path?.replace(".srt", ".synced.srt"),
      });
      if (selected) {
        await invoke<string>("sync_save_file", { outputPath: selected });
        showSnackbar(`${t("sync.fileSaved")} ${selected}`, "success");
        addSyncLog(`Saved synced file: ${getFileName(selected)}`, "success");
      }
    } catch (e) {
      error = `${t("sync.errorSaving")} ${e}`;
      addSyncLog(`Save file failed: ${e}`, "error");
    }
  }

  async function saveSession() {
    console.debug("[SyncTab] saveSession");
    try {
      const selected = await guardedSave({
        filters: [{ name: "Session Files", extensions: ["json"] }],
      });
      if (selected) {
        await invoke<string>("sync_save_session", { sessionPath: selected });
        showSnackbar(`${t("sync.sessionSaved")} ${selected}`, "success");
        addSyncLog(`Session saved: ${getFileName(selected)}`, "success");
      }
    } catch (e) {
      error = `${t("sync.errorSaving")} ${e}`;
      addSyncLog(`Save session failed: ${e}`, "error");
    }
  }

  async function loadSession() {
    console.debug("[SyncTab] loadSession");
    try {
      const selected = await guardedOpen({
        filters: [{ name: "Session Files", extensions: ["json"] }],
      });
      if (selected) {
        status = await invoke<SyncStatus>("sync_load_session", {
          sessionPath: selected as string,
        });
        addSyncLog(`Session loaded: ${getFileName(selected as string)}`, "success");
        await loadSubtitles();
        await loadAnchors();
        wizardHistory = anchors.map((a) => a.subtitle_id);
        await advanceWizard();
      }
    } catch (e) {
      error = `${t("sync.errorLoadingSrt")} ${e}`;
      addSyncLog(`Load session failed: ${e}`, "error");
    }
  }

  async function confirmReset() {
    showResetModal = false;
    console.debug("[SyncTab] confirmReset");
    try {
      status = await invoke<SyncStatus>("sync_reset");
      cleanupAudioSrc();
      audioError = null;
      subtitles = [];
      anchors = [];
      wizardSubtitle = null;
      wizardHistory = [];
      showSaveSuggestion = false;
      currentVideoTime = 0;
      isPlaying = false;
      resetOffset();
      currentPage = 1;
      manualGoToId = "";
      if (audioElement) {
        audioElement.pause();
        audioElement.src = "";
      }
      addSyncLog("Session reset", "warning");
    } catch (e) {
      error = `${t("sync.errorSaving")} ${e}`;
      addSyncLog(`Reset failed: ${e}`, "error");
    }
  }

  function isSrtFile(name: string): boolean {
    return name.toLowerCase().endsWith(".srt");
  }
  function isMediaFile(name: string): boolean {
    const ext = name.toLowerCase().split(".").pop() || "";
    return [
      "mp4",
      "mkv",
      "avi",
      "webm",
      "mov",
      "m4v",
      "m2ts",
      "mpeg",
      "mpg",
      "mp3",
      "wav",
      "ogg",
      "flac",
      "m4a",
      "aac",
      "wma",
      "opus",
      "m4b",
    ].includes(ext);
  }

  async function handleDroppedPaths(paths: string[]) {
    if (paths.length === 0) return;

    // Sort paths so SRT files are processed first
    const sortedPaths = [...paths].sort((a, b) => {
      const isASrt = isSrtFile(a);
      const isBSrt = isSrtFile(b);
      if (isASrt && !isBSrt) return -1;
      if (!isASrt && isBSrt) return 1;
      return 0;
    });

    for (const filePath of sortedPaths) {
      const fileName = getFileName(filePath);
      if (isSrtFile(fileName)) {
        try {
          status = await invoke<SyncStatus>("sync_load_srt", {
            path: filePath,
          });
          addSyncLog(`Dropped SRT: ${fileName}`, "success");
          await loadSubtitles();
          await loadAnchors();
          wizardHistory = [];
          showSaveSuggestion = false;
          await advanceWizard();
          await tryAutoSelectMediaForSrt(filePath);
        } catch (e: any) {
          error = `${t("sync.errorLoadingSrt")} ${e}`;
          addSyncLog(`Dropped SRT failed: ${e}`, "error");
        }
      } else if (isMediaFile(fileName)) {
        if (!status?.is_loaded) {
          showSnackbar(t("sync.dropSrtFirst"), "warning");
          return;
        }
        try {
          audioError = null;
          cleanupAudioSrc();
          audioSrc = await loadMediaFile(filePath);
          status = await invoke<SyncStatus>("sync_set_video", {
            path: filePath,
          });
          addSyncLog(`Dropped media: ${fileName}`, "success");
        } catch (e: any) {
          error = `${t("sync.errorLoadingAudio")} ${e}`;
          addSyncLog(`Dropped media failed: ${e}`, "error");
        }
      }
    }
  }

  async function undoLastAnchor() {
    if (anchors.length === 0) return;
    // Remove the most recently *created* anchor? Actually anchors might be sorted by original_time_ms.
    // Let's just remove the anchor for the last subtitle we visited, or the highest ID anchor if we just advanced.
    // A simpler approach: find the anchor with the highest subtitle_id that we recently added, or just pop the last anchor.
    // If wizardHistory has it, pop it.
    const lastHistoryId = wizardHistory.length > 0 ? wizardHistory[wizardHistory.length - 1] : null;
    const anchorToRemove = anchors.find(a => a.subtitle_id === lastHistoryId) || anchors[anchors.length - 1];
    
    if (!anchorToRemove) return;

    try {
      status = await invoke<SyncStatus>("sync_remove_anchor", { subtitleId: anchorToRemove.subtitle_id });
      wizardHistory = wizardHistory.filter(id => id !== anchorToRemove.subtitle_id);
      
      await loadAnchors();
      await refreshCurrentSubtitles();
      
      addSyncLog(`Undo: Removed anchor #${anchorToRemove.subtitle_id}`, "warning");
      
      // Go back to that subtitle to re-adjust
      const sub = await invoke<SubtitleInfo>("sync_get_subtitle", { id: anchorToRemove.subtitle_id });
      wizardSubtitle = sub;
      resetOffset();
      seekToSubtitleStart(sub);
      scrollToSubtitle(sub.id);
    } catch (e) {
      error = `Undo failed: ${e}`;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (
      document.activeElement?.tagName === "INPUT" ||
      document.activeElement?.tagName === "TEXTAREA"
    )
      return;

    // Undo action (Ctrl+Z)
    if (e.key === "z" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      undoLastAnchor();
      return;
    }

    // Open SRT action (Ctrl+O)
    if (e.key === "o" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      selectSrtFile();
      return;
    }

    // Auto-Sync (Ctrl+A)
    if (e.key === "a" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      if (hasAudio) startAutoSync();
      return;
    }

    // New Sync (Ctrl+N)
    if (e.key === "n" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      if (hasAudio) showResetModal = true;
      return;
    }

    // Load Session (Ctrl+L)
    if (e.key === "l" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault();
      loadSession();
      return;
    }

    // Save Session (Ctrl+Shift+S)
    if (e.key === "S" && (e.ctrlKey || e.metaKey) && e.shiftKey) {
      e.preventDefault();
      if (hasAudio) saveSession();
      return;
    }


    switch (e.key) {
      case " ":
        e.preventDefault();
        if (hasAudio && audioElement) {
          if (isPlaying) {
            audioElement.pause();
          } else {
            void safePlayAudio("keyboard-space");
          }
        }
        break;
      case "ArrowLeft":
        e.preventDefault();
        if (hasAudio && audioElement)
          audioElement.currentTime -= e.shiftKey ? 3 : 0.5;
        break;
      case "ArrowRight":
        e.preventDefault();
        if (hasAudio && audioElement)
          audioElement.currentTime += e.shiftKey ? 3 : 0.5;
        break;
      case "ArrowUp":
        if (wizardSubtitle) {
          e.preventDefault();
          updateOffset(e.altKey ? 3000 : e.shiftKey ? 500 : 100);
          seekToSubtitleStart(wizardSubtitle);
        }
        break;
      case "ArrowDown":
        if (wizardSubtitle) {
          e.preventDefault();
          updateOffset(-(e.altKey ? 3000 : e.shiftKey ? 500 : 100));
          seekToSubtitleStart(wizardSubtitle);
        }
        break;
      case "Enter":
        e.preventDefault();
        confirmCurrentCheckpoint();
        break;
      case "r":
      case "R":
        e.preventDefault();
        replayCurrentSubtitle();
        break;
    }
  }

  let singleClickTimer: ReturnType<typeof setTimeout> | null = null;

  function handleSubtitleClick(sub: SubtitleInfo) {
    if (singleClickTimer) {
      clearTimeout(singleClickTimer);
      singleClickTimer = null;
    }
    singleClickTimer = setTimeout(() => {
      singleClickTimer = null;
      goToSubtitleManual(sub);
    }, 200);
  }

  function handleSubtitleDblClick(sub: SubtitleInfo) {
    if (singleClickTimer) {
      clearTimeout(singleClickTimer);
      singleClickTimer = null;
    }
    playSubtitleFromList(sub);
  }

  onMount(() => {
    syncDebug("mount", { active });
    window.addEventListener("keydown", handleKeydown);
    let unlistenDragDrop: (() => void) | null = null;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (!active) return;
        if (event.payload.type === "over") {
          isDraggingOver = true;
        }
        else if (event.payload.type === "drop") {
          isDraggingOver = false;
          syncDebug("file-drop", { count: event.payload.paths.length });
          handleDroppedPaths(event.payload.paths);
        } else if (event.payload.type === "leave") isDraggingOver = false;
      })
      .then((fn) => {
        unlistenDragDrop = fn;
      });
    return () => {
      syncDebug("unmount");
      window.removeEventListener("keydown", handleKeydown);
      if (unlistenDragDrop) unlistenDragDrop();
      if (singleClickTimer) clearTimeout(singleClickTimer);
    };
  });
</script>

<div
  class="h-full flex flex-col p-6 overflow-y-auto bg-gradient-to-br from-gray-900 via-gray-900 to-gray-950 relative"
  role="application"
  ondrop={(e) => {
    e.preventDefault();
    if (e.dataTransfer?.types.includes("Files")) {
      isDraggingOver = false;
    }
  }}
  ondragover={(e) => {
    e.preventDefault();
    if (e.dataTransfer?.types.includes("Files")) {
      isDraggingOver = true;
    }
  }}
  ondragleave={(e) => {
    const rt = e.relatedTarget as HTMLElement | null;
    const ct = e.currentTarget as HTMLElement;
    if (rt && ct.contains(rt)) return;
    if (e.dataTransfer?.types.includes("Files")) {
      isDraggingOver = false;
    }
  }}
>
  {#if isDraggingOver}
    <div
      class="absolute inset-0 z-50 bg-indigo-500/10 border-2 border-dashed border-indigo-400 rounded-2xl flex items-center justify-center pointer-events-none"
    >
      <div class="text-center">
        <svg
          class="w-16 h-16 mx-auto mb-3 text-indigo-400"
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
        <p class="text-lg font-medium text-indigo-300">
          {t("sync.dropFileHere")}
        </p>
        <p class="text-sm text-gray-400 mt-1">{t("sync.dropFileHint")}</p>
      </div>
    </div>
  {/if}

  <!-- svelte-ignore a11y_media_has_caption -->
  <audio
    bind:this={audioElement}
    src={audioSrc || undefined}
    class="hidden"
    preload="metadata"
    ontimeupdate={() => {
      if (audioElement) {
        currentVideoTime = audioElement.currentTime;
        if (wizardSubtitle && !audioElement.paused && !hasAutoPaused) {
          const endSec =
            (wizardSubtitle.synced_end_ms + offsetAdjustment) / 1000;
          if (audioElement.currentTime >= endSec + 0.5) {
            audioElement.pause();
            hasAutoPaused = true;
          }
        }
      }
    }}
    onplay={() => {
      isPlaying = true;
      syncDebug("audio:onplay", {
        currentTime: audioElement?.currentTime,
        duration: audioElement?.duration,
      });
    }}
    onpause={() => {
      isPlaying = false;
      syncDebug("audio:onpause", {
        currentTime: audioElement?.currentTime,
      });
    }}
    onloadedmetadata={() => {
      if (audioElement) audioDuration = audioElement.duration;
      audioError = null;
      syncDebug("audio:onloadedmetadata", {
        duration: audioElement?.duration,
        src: audioSrc,
      });
    }}
    onerror={(e) => {
      const el = e.currentTarget as HTMLMediaElement;
      const mediaErr = el?.error;
      const codeMap: Record<number, string> = {
        1: "MEDIA_ERR_ABORTED",
        2: "MEDIA_ERR_NETWORK",
        3: "MEDIA_ERR_DECODE",
        4: "MEDIA_ERR_SRC_NOT_SUPPORTED",
      };
      const code = mediaErr?.code || 0;
      const codeStr = codeMap[code] || `Unknown error: ${code}`;
      const msg = mediaErr?.message || "";
      const gstMissingSink = /autoaudiosink|audiosink/i.test(msg);
      if (gstMissingSink) {
        audioError = `${codeStr}. Audio backend non disponibile su Linux. Installa almeno gstreamer1.0-plugins-good e, se necessario, gstreamer1.0-pulseaudio o pipewire-audio, poi riavvia l'app. ${msg}`;
      } else if (code === 3 || code === 4) {
        audioError = `${codeStr}. Su Linux potrebbe servire: gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav. ${msg}`;
      } else {
        audioError = `${codeStr}. ${msg}`;
      }
      syncDebug("audio:onerror", {
        code,
        codeStr,
        message: msg,
      });
    }}
    oncanplay={() => {
      audioError = null;
      syncDebug("audio:oncanplay", {
        currentTime: audioElement?.currentTime,
        readyState: audioElement?.readyState,
      });
    }}
    onseeking={() => {
      syncDebug("audio:onseeking", { currentTime: audioElement?.currentTime });
    }}
    onseeked={() => {
      syncDebug("audio:onseeked", { currentTime: audioElement?.currentTime });
    }}
  ></audio>

  {#snippet panelContent(panelId: SyncPanelId)}
    {#if panelId === "toolbar"}
      <div class="glass-card flex flex-col gap-3 p-4 flex-shrink-0">
        <!-- Row 1: File inputs -->
        <div class="flex items-center gap-2 w-full">
          <div class="flex-1 min-w-0">
            <div class="flex gap-2">
              <button
                type="button"
                onclick={() => {
                  if (status?.srt_path) expandedPathField = "srt";
                }}
                class="input-modern flex-1 text-sm text-left transition-colors truncate {status?.srt_path
                  ? 'cursor-pointer hover:bg-white/10'
                  : 'cursor-default hover:bg-transparent'}"
                style="direction: rtl; text-align: left;"
                title={status?.srt_path || t("sync.noSrt")}
              >
                <span
                  class={status?.srt_path ? "text-white" : "text-gray-500"}
                  style="unicode-bidi: plaintext;"
                >
                  {status?.srt_path || t("sync.loadSrt")}
                </span>
              </button>
              <button
                onclick={selectSrtFile}
                class="btn-primary py-2 px-3 flex items-center justify-center"
                title={t("sync.tooltip.loadSrt")}
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
              </button>
            </div>
          </div>

          <div class="text-gray-500 flex-shrink-0 {status?.is_loaded ? 'text-indigo-400' : ''}">
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M13 7l5 5m0 0l-5 5m5-5H6"
              /></svg
            >
          </div>

          <div class="flex-1 min-w-0">
            <div class="flex gap-2">
              <button
                type="button"
                onclick={() => {
                  if (status?.video_path) expandedPathField = "media";
                }}
                class="input-modern flex-1 text-sm text-left transition-colors truncate {status?.video_path
                  ? 'cursor-pointer hover:bg-white/10'
                  : 'cursor-default hover:bg-transparent'} {!status?.is_loaded ? 'opacity-60' : ''}"
                style="direction: rtl; text-align: left;"
                title={status?.video_path || t("sync.noVideo")}
              >
                <span
                  class={status?.video_path ? "text-white" : "text-gray-500"}
                  style="unicode-bidi: plaintext;"
                >
                  {status?.video_path || t("sync.loadAudio")}
                </span>
              </button>
              <button
                onclick={selectAudioFile}
                disabled={!status?.is_loaded}
                class="btn-secondary py-2 px-3 disabled:opacity-30 disabled:cursor-not-allowed"
                title={t("sync.tooltip.loadVideo")}
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
                    d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                  />
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <!-- Row 2: Action buttons -->
        <div class="flex items-center gap-2 w-full">
          <div class="relative group flex-1">
            <button
              type="button"
              onclick={startAutoSync}
              disabled={isAutoSyncing || !status?.is_loaded || !hasAudio}
              class="w-full h-10 flex items-center justify-center gap-2 rounded-lg border bg-indigo-500/20 border-indigo-500/40 text-indigo-300 hover:bg-indigo-500/30 text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {#if isAutoSyncing}
                <svg class="animate-spin w-4 h-4 text-indigo-300" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
              {:else}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" /></svg>
              {/if}
              {t("sync.autoSync")}
            </button>
            {#if !status?.is_loaded || !hasAudio}
              <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-1.5 bg-gray-800 border border-white/10 text-xs text-indigo-300 rounded-lg shadow-xl opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
                {t("sync.autoSyncRequires")}
              </div>
            {/if}
          </div>

          {#if status?.is_loaded || audioSrc}
            <button
              onclick={() => (showResetModal = true)}
              class="flex-1 h-10 flex items-center justify-center gap-2 rounded-lg border bg-amber-500/20 border-amber-500/40 text-amber-300 hover:bg-amber-500/30 text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              title={t("sync.newSyncDesc")}
            >
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                ><path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                /></svg
              >
              {t("sync.newSync")}
            </button>
          {/if}

          <button
            onclick={loadSession}
            class="flex-1 h-10 flex items-center justify-center gap-2 rounded-lg border bg-cyan-500/20 border-cyan-500/40 text-cyan-300 hover:bg-cyan-500/30 text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            title={t("sync.tooltipLoadSession")}
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
              /></svg
            >
            {t("sync.loadSession")}
          </button>

          <button
            onclick={saveSession}
            disabled={!status?.is_loaded}
            class="flex-1 h-10 flex items-center justify-center gap-2 rounded-lg border bg-teal-500/20 border-teal-500/40 text-teal-300 hover:bg-teal-500/30 text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            title={t("sync.tooltipSaveSession")}
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"
              /></svg
            >
            {t("sync.saveSession")}
          </button>

          <button
            onclick={saveFile}
            disabled={!status?.is_loaded}
            class="flex-1 h-10 flex items-center justify-center gap-2 rounded-lg border bg-green-500/20 border-green-500/40 text-green-300 hover:bg-green-500/30 text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed"
            title={t("sync.tooltipSaveFile")}
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M5 13l4 4L19 7"
              /></svg
            >
            {t("sync.saveFile")}
          </button>
        </div>
      </div>
    {:else if panelId === "wizard"}
      <div class="glass-card relative flex flex-col h-full overflow-hidden">
        <div class="p-3 flex items-center gap-2 flex-shrink-0">
          <svg
            class="w-5 h-5 text-indigo-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
            /></svg
          >
          <h3 class="text-sm font-semibold text-indigo-400">
            {t("sync.wizard.title")}
          </h3>
          <InfoButton onclick={() => (helpSection = "wizard")} />
        </div>

        <div
          class="flex-1 flex flex-col items-center justify-center p-6 min-h-0 overflow-y-auto"
        >
          {#if !status?.is_loaded}
            <div class="text-gray-500 text-center">
              <svg
                class="w-20 h-20 mx-auto mb-4 opacity-50"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                ><path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                /></svg
              >
              <p class="text-lg">{t("sync.srtPlaceholder")}</p>
            </div>
          {:else if showSaveSuggestion}
            <div class="text-center max-w-md">
              <div
                class="w-20 h-20 mx-auto mb-6 rounded-full bg-gradient-to-br from-green-500/20 to-emerald-500/20 flex items-center justify-center"
              >
                <svg
                  class="w-10 h-10 text-green-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  ><path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 13l4 4L19 7"
                  /></svg
                >
              </div>
              <h3 class="text-2xl font-bold text-white mb-3">
                {t("sync.wizard.allSynced")}
              </h3>
              <p class="text-gray-400 mb-6">{t("sync.wizard.suggestSave")}</p>
              <div class="flex gap-3 justify-center">
                <button
                  onclick={saveFile}
                  class="btn-success py-3 px-8 flex items-center gap-2 text-lg shadow-lg shadow-green-500/30"
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    /></svg
                  >
                  {t("sync.saveFile")}
                </button>
                <button
                  onclick={() => {
                    showSaveSuggestion = false;
                  }}
                  class="btn-secondary py-3 px-6"
                >
                  {t("sync.wizard.continueChecking")}
                </button>
              </div>
            </div>
          {:else if wizardSubtitle}
            <div class="w-full max-w-2xl flex flex-col gap-4">
              <div class="text-center flex-shrink-0">
                <span
                  class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-indigo-500/20 text-indigo-300 text-sm font-medium"
                >
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
                    /></svg
                  >
                  {t("sync.wizard.checkpoint")} — #{wizardSubtitle.id} / {status?.total_subtitles}
                </span>
              </div>

              <div
                class="bg-white/5 rounded-2xl p-6 text-center flex-shrink-0 flex flex-col items-center justify-center"
                style="min-height: 120px;"
              >
                <p class="text-2xl text-white font-medium leading-relaxed">
                  {wizardSubtitle.text}
                </p>
                <p class="text-sm text-gray-500 mt-3 font-mono">
                  {formatTime(wizardSubtitle.start_ms)} → {formatTime(
                    wizardSubtitle.end_ms,
                  )}
                </p>
              </div>

              <div class="flex-shrink-0">
                {#if audioSrc && !audioError}
                  <div class="flex items-center gap-4">
                    <span class="text-sm text-gray-400 font-mono w-24"
                      >{formatTime(currentVideoTime * 1000)}</span
                    >
                    <input
                      type="range"
                      min="0"
                      max={audioDuration || 100}
                      step="0.01"
                      bind:value={currentVideoTime}
                      oninput={() => {
                        if (audioElement)
                          audioElement.currentTime = currentVideoTime;
                      }}
                      class="flex-1"
                    />
                    <span
                      class="text-sm text-gray-400 font-mono w-24 text-right"
                      >{audioDuration
                        ? formatTime(audioDuration * 1000)
                        : "--:--"}</span
                    >
                  </div>
                {:else if !audioSrc}
                  <div class="text-center py-4">
                    <p class="text-gray-500">{t("sync.audioPlaceholder")}</p>
                    <p class="text-xs text-gray-600 mt-1">
                      {t("sync.audioFormats")}
                    </p>
                  </div>
                {:else if audioError}
                  <div class="text-center py-4 max-w-xl mx-auto">
                    <p
                      class="text-red-400 text-sm whitespace-pre-wrap break-words"
                    >
                      {audioError}
                    </p>
                    <button
                      onclick={() => {
                        audioError = null;
                        audioSrc = null;
                        selectAudioFile();
                      }}
                      class="btn-secondary text-sm mt-2"
                      >{t("sync.tryAnotherFile")}</button
                    >
                  </div>
                {/if}
              </div>

              <div
                class="flex items-center justify-center gap-4 flex-wrap flex-shrink-0"
                style="min-height: 60px;"
              >
                <button
                  onclick={() =>
                    hasAudio &&
                    audioElement &&
                    (isPlaying
                      ? audioElement.pause()
                      : void safePlayAudio("wizard-play-button"))}
                  disabled={!hasAudio}
                  class="w-14 h-14 flex items-center justify-center rounded-full bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-700 hover:to-purple-700 shadow-lg shadow-indigo-500/30 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
                  title={isPlaying ? t("sync.tooltipPause") : t("sync.tooltipPlay")}
                >
                  {#if isPlaying}
                    <svg
                      class="w-7 h-7 text-white"
                      fill="currentColor"
                      viewBox="0 0 24 24"
                      ><path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" /></svg
                    >
                  {:else}
                    <svg
                      class="w-7 h-7 text-white ml-1"
                      fill="currentColor"
                      viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg
                    >
                  {/if}
                </button>

                <button
                  onclick={replayCurrentSubtitle}
                  disabled={!hasAudio}
                  class="w-12 h-12 flex items-center justify-center rounded-full bg-white/10 hover:bg-white/20 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                  title={t("sync.wizard.replay") + " (R)"}
                >
                  <svg
                    class="w-6 h-6 text-cyan-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                    /></svg
                  >
                </button>

                <div
                  class="flex items-center gap-2 bg-white/5 rounded-xl px-4 py-2 {!hasAudio
                    ? 'opacity-40 pointer-events-none'
                    : ''}"
                >
                  <button
                    onclick={() => {
                      updateOffset(-3000);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="-3s"
                  >
                    −3s
                  </button>
                  <button
                    onclick={() => {
                      updateOffset(-500);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="-0.5s"
                  >
                    −0.5s
                  </button>
                  <button
                    onclick={() => {
                      updateOffset(-100);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-8 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-lg font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="-100ms"
                  >
                    −
                  </button>
                  <div class="flex flex-col items-center min-w-[80px]">
                    <span class="text-xs text-gray-500 uppercase tracking-wide"
                      >{t("sync.offset")}</span
                    >
                    <span
                      class="text-base font-mono font-medium {offsetAdjustment >
                      0
                        ? 'text-green-400'
                        : offsetAdjustment < 0
                          ? 'text-red-400'
                          : 'text-white'}"
                      >{formatOffset(offsetAdjustment)}</span
                    >
                  </div>
                  <button
                    onclick={() => {
                      updateOffset(100);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-8 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-lg font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="+100ms"
                  >
                    +
                  </button>
                  <button
                    onclick={() => {
                      updateOffset(500);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="+0.5s"
                  >
                    +0.5s
                  </button>
                  <button
                    onclick={() => {
                      updateOffset(3000);
                      replayCurrentSubtitle();
                    }}
                    disabled={!hasAudio}
                    class="w-12 h-8 flex items-center justify-center bg-white/10 hover:bg-white/20 rounded-lg text-[11px] font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    title="+3s"
                  >
                    +3s
                  </button>
                </div>

                <button
                  onclick={skipCheckpoint}
                  class="btn-secondary py-3 px-6 flex items-center gap-2 text-base font-medium"
                  title={t("sync.wizard.skip")}
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
                  {t("sync.wizard.skip")}
                </button>

                <button
                  onclick={confirmCurrentCheckpoint}
                  disabled={!audioSrc || !!audioError}
                  class="btn-success py-3 px-6 flex items-center gap-2 disabled:opacity-50 shadow-lg shadow-green-500/20 text-base font-medium"
                >
                  <svg
                    class="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    /></svg
                  >
                  {t("sync.wizard.confirm")}
                </button>
              </div>

              <div
                class="flex flex-wrap gap-3 text-xs text-gray-500 justify-center flex-shrink-0"
              >
                <div class="flex items-center gap-1">
                  <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
                    >Space</kbd
                  ><span>{t("sync.playPause")}</span>
                </div>
                <div class="flex items-center gap-1">
                  <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
                    >←/→</kbd
                  ><span>{t("sync.seek")}</span>
                </div>
                <div class="flex items-center gap-1">
                  <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
                    >↑/↓</kbd
                  ><span>{t("sync.offsetAdjust")}</span>
                </div>
                <div class="flex items-center gap-1">
                  <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
                    >R</kbd
                  ><span>{t("sync.wizard.replay")}</span>
                </div>
                <div class="flex items-center gap-1">
                  <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
                    >Enter</kbd
                  ><span>{t("sync.confirm")}</span>
                </div>
              </div>
            </div>
          {:else}
            <div class="text-gray-500 text-center">
              <p class="text-lg">{t("sync.wizard.selectCheckpoint")}</p>
            </div>
          {/if}
        </div>
      </div>
    {:else if panelId === "status"}
      <div class="glass-card p-4 h-full min-h-0 overflow-y-auto space-y-4">
        <div class="flex items-center gap-2">
          <svg
            class="w-5 h-5 text-cyan-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
            /></svg
          >
          <h3 class="text-sm font-semibold text-cyan-400">
            {t("sync.statusTitle")}
          </h3>
          <InfoButton onclick={() => (helpSection = "status")} />
        </div>

        {#if status?.is_loaded}
          <div class="grid grid-cols-2 gap-3">
            <div class="bg-white/5 rounded-xl p-3 text-center">
              <p class="text-2xl font-bold text-white">
                {status.total_subtitles}
              </p>
              <p class="text-xs text-gray-500">{t("sync.subtitles")}</p>
            </div>
            <div class="bg-white/5 rounded-xl p-3 text-center">
              <p class="text-2xl font-bold text-green-400">
                {status.anchor_count}
              </p>
              <p class="text-xs text-gray-500">{t("sync.anchors")}</p>
            </div>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-400">{t("sync.averageOffset")}:</span>
              <span
                class={status.average_offset_ms > 0
                  ? "text-green-400"
                  : status.average_offset_ms < 0
                    ? "text-red-400"
                    : "text-white"}
                >{formatOffset(status.average_offset_ms)}</span
              >
            </div>
            <div class="progress-modern h-2">
              <div
                class="progress-modern-bar"
                style="width: {status.completion_percentage}%"
              ></div>
            </div>
            <div class="flex justify-between items-center text-xs text-gray-500 px-1">
              <span>{status.completion_percentage.toFixed(1)}% {t("sync.completed")}</span>
              <span class="text-indigo-400 font-medium" title={t("sync.confidenceHelp") || "Confidence based on anchors"}>{t("sync.confidence") || "Confidence"}: {confidenceScore}%</span>
            </div>
          </div>

          <div class="flex gap-2 items-center">
            <input
              type="number"
              min="1"
              max={status.total_subtitles}
              bind:value={manualGoToId}
              placeholder={t("sync.wizard.goToLine")}
              class="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white placeholder-gray-600 focus:outline-none focus:border-indigo-500"
              onkeydown={(e) => {
                if (e.key === "Enter") goToLineById();
              }}
            />
            <button
              onclick={goToLineById}
              class="btn-secondary py-1.5 px-3 text-sm"
              >{t("sync.wizard.go")}</button
            >
          </div>

          {#if anchors.length > 0}
            <div class="border-t border-white/10 pt-3">
              <h4
                class="text-sm font-semibold text-indigo-400 mb-2 flex items-center gap-2"
              >
                <svg
                  class="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  ><path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"
                  /></svg
                >
                {t("sync.anchors")} ({anchors.length})
                <span class="text-[11px] font-normal text-gray-500">
                  {manualAnchors.length} manual
                </span>
              </h4>
              <div class="space-y-2 max-h-56 overflow-y-auto pr-1">
                {#each anchors as anchor}
                  <div
                    class="flex items-center justify-between text-sm bg-white/5 rounded-lg px-3 py-2"
                  >
                    <button
                      onclick={() => goToCheckpoint(anchor.subtitle_id)}
                      class="text-gray-400 hover:text-indigo-300 transition-colors"
                      >#{anchor.subtitle_id}</button
                    >
                    <span
                      class={anchor.offset_ms >= 0
                        ? "text-green-400"
                        : "text-red-400"}>{formatOffset(anchor.offset_ms)}</span
                    >
                    <span class={anchor.is_manual ? "text-emerald-300 text-[11px]" : "text-amber-300 text-[11px]"}>
                      {anchor.is_manual ? "manual" : "auto"}
                    </span>
                    <button
                      onclick={() => removeAnchor(anchor.subtitle_id)}
                      class="text-red-400 hover:text-red-300 p-1 hover:bg-red-500/20 rounded transition-colors"
                      aria-label={t("sync.tooltipRemoveAnchor")}
                    >
                      <svg
                        class="w-4 h-4"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        ><path
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          stroke-width="2"
                          d="M6 18L18 6M6 6l12 12"
                        /></svg
                      >
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <div class="border-t border-white/10 pt-3">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-semibold text-cyan-300">Activity Log</h4>
              {#if syncLogs.length > 0}
                <button
                  onclick={clearSyncLogs}
                  class="text-xs text-gray-500 hover:text-gray-300 transition-colors"
                >Clear</button>
              {/if}
            </div>
            <div class="space-y-1.5 pr-1 max-h-52 overflow-y-auto">
              {#if syncLogs.length > 0}
                {#each syncLogs as entry (entry.id)}
                  <div class="rounded-lg border px-2.5 py-1.5 text-xs flex items-start gap-2 {entry.level ===
                  'success'
                    ? 'bg-green-500/10 border-green-500/25 text-green-300'
                    : entry.level === 'warning'
                      ? 'bg-amber-500/10 border-amber-500/25 text-amber-300'
                      : entry.level === 'error'
                        ? 'bg-red-500/10 border-red-500/25 text-red-300'
                        : 'bg-white/5 border-white/10 text-gray-300'}">
                    <span class="text-[10px] text-gray-500 mt-0.5">{entry.timestamp}</span>
                    <span class="leading-snug">{entry.message}</span>
                  </div>
                {/each}
              {:else}
                <p class="text-xs text-gray-500">{t("sync.noLog")}</p>
              {/if}
            </div>
          </div>
        {:else}
          <p class="text-gray-500 text-sm text-center py-4">
            {t("sync.srtPlaceholder")}
          </p>
        {/if}
      </div>
    {:else if panelId === "subtitleList"}
      <div class="glass-card flex flex-col h-full min-h-0">
        <!-- Header -->
        <div class="p-4 pb-2 flex-shrink-0 flex items-center gap-2">
          <svg
            class="w-4 h-4 text-purple-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 6h16M4 10h16M4 14h16M4 18h16"
            /></svg
          >
          <h4 class="text-sm font-semibold text-purple-400">
            {t("sync.subtitles")}
            {#if status?.is_loaded}<span class="text-gray-500 font-normal"
                >(Page {currentPage} of {totalPages})</span
              >{/if}
          </h4>
          <InfoButton onclick={() => (helpSection = "subtitleList")} />
        </div>

        <!-- Pagination controls — TOP -->
        {#if status?.is_loaded && totalPages > 1}
          <div class="px-3 pb-2 flex items-center justify-between flex-shrink-0">
            <button
              onclick={() => loadPage(currentPage - 1)}
              disabled={currentPage <= 1}
              class="btn-secondary py-1 px-2.5 text-xs flex items-center gap-1 disabled:opacity-30"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/></svg>
              {t("sync.pagination.prev") || "Prev"}
            </button>
            <span class="text-xs text-gray-400">Page {currentPage} / {totalPages}</span>
            <button
              onclick={() => loadPage(currentPage + 1)}
              disabled={currentPage >= totalPages}
              class="btn-secondary py-1 px-2.5 text-xs flex items-center gap-1 disabled:opacity-30"
            >
              {t("sync.pagination.next") || "Next"}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/></svg>
            </button>
          </div>
        {/if}

        <!-- Subtitle list -->
        <div
          class="flex flex-col overflow-y-auto flex-1 p-2 min-h-0"
          bind:this={subtitleListElement}
        >
          {#each subtitles as sub (sub.id)}
            <button
              onclick={() => handleSubtitleClick(sub)}
              ondblclick={() => handleSubtitleDblClick(sub)}
              oncontextmenu={(e) => openSubtitleContextMenu(e, sub)}
              data-subtitle-id={sub.id}
              class="w-full text-left p-3 border-b border-white/5 hover:bg-white/5
                {wizardSubtitle?.id === sub.id
                ? 'bg-indigo-500/20 border-l-4 border-l-indigo-500'
                : ''}
                {sub.is_anchor ? 'bg-green-500/5' : ''}"
            >
              <div class="flex items-start gap-2">
                <span class="text-xs text-gray-500 w-8 flex-shrink-0"
                  >#{sub.id}</span
                >
                <div class="flex-1 min-w-0">
                  <p class="text-sm truncate text-gray-200">{sub.text}</p>
                  <div class="flex gap-2 text-xs text-gray-500 mt-1">
                    <span class="font-mono"
                      >{formatTime(sub.synced_start_ms)}</span
                    >
                    <span class="text-gray-700">→</span>
                    <span class="font-mono"
                      >{formatTime(sub.synced_end_ms)}</span
                    >
                    {#if sub.offset_ms !== 0}<span
                        class={sub.offset_ms > 0
                          ? "text-green-400"
                          : "text-red-400"}>{formatOffset(sub.offset_ms)}</span
                      >{/if}
                  </div>
                </div>
                {#if sub.is_anchor}
                  <div
                    role="button"
                    tabindex="0"
                    onclick={(e) => {
                      e.stopPropagation();
                      removeAnchor(sub.id);
                    }}
                    onkeydown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.stopPropagation();
                        e.preventDefault();
                        removeAnchor(sub.id);
                      }
                    }}
                    class="text-green-400 hover:text-red-400 transition-colors flex-shrink-0 p-1 rounded hover:bg-white/5 cursor-pointer"
                    title={t("sync.tooltipRemoveAnchor")}
                  >
                    <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/></svg>
                  </div>
                {/if}
              </div>
            </button>
          {/each}
          {#if subtitles.length === 0 && !status?.is_loaded}
            <div class="text-center text-gray-500 py-12">
              <svg
                class="w-12 h-12 mx-auto mb-4 opacity-50"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                ><path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                /></svg
              >
              <p>{t("sync.srtPlaceholder")}</p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {/snippet}

  <div class="mb-3">
    {@render panelContent("toolbar")}
  </div>

  <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
    <!-- Left column: wizard -->
    <div class="flex flex-col" role="list">
      <div class="flex flex-col" role="listitem">
        {@render panelContent("wizard")}
      </div>
    </div>

    <!-- Right column: status + subtitle list (inner scroll per panel) -->
    <div class="flex flex-col gap-3" role="list">
      <div class="flex flex-col min-h-[220px]" role="listitem">
        {@render panelContent("status")}
      </div>
      <div class="flex flex-col min-h-[240px]" role="listitem">
        {@render panelContent("subtitleList")}
      </div>
    </div>
  </div>

  {#if isAutoSyncing}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[100] bg-black/80 flex items-center justify-center p-6 backdrop-blur-sm"
         onclick={(e) => e.stopPropagation()}
         onkeydown={(e) => e.stopPropagation()}
    >
      <div class="max-w-md w-full p-8 text-center flex flex-col items-center bg-[#0f172a] border border-indigo-300/20 rounded-2xl shadow-2xl opacity-100">
        <svg class="animate-spin w-12 h-12 text-indigo-400 mb-6" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
        <h3 class="text-xl font-bold text-white mb-2">{t("sync.autoSyncInProgress")}</h3>
        <p class="text-indigo-300 text-sm mb-6 max-w-[280px] leading-relaxed">{autoSyncMessage}</p>
        
        <div class="w-full bg-gray-800 rounded-full h-3 mb-2 overflow-hidden border border-white/5">
          <div
            class="bg-indigo-500 h-full rounded-full transition-all duration-300 ease-out relative overflow-hidden"
            style="width: {autoSyncProgress}%"
          >
            <div class="absolute inset-0 bg-white/20 animate-pulse"></div>
          </div>
        </div>
        <p class="text-gray-400 text-xs font-mono">{Math.round(autoSyncProgress)}%</p>
        <button
          onclick={cancelAutoSync}
          disabled={isCancellingAutoSync}
          class="mt-6 px-4 py-2 border border-red-500/50 text-red-400 hover:bg-red-500/20 rounded-lg text-sm transition-colors disabled:opacity-50"
        >
          {isCancellingAutoSync ? t("sync.autoSyncCancelling") : t("sync.autoSyncCancel")}
        </button>
      </div>
    </div>
  {/if}

  {#if subtitleContextMenu}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onclick={closeSubtitleContextMenu}
      oncontextmenu={(e) => {
        e.preventDefault();
        closeSubtitleContextMenu();
      }}
      onkeydown={(e) => {
        if (e.key === "Escape") closeSubtitleContextMenu();
      }}
      role="presentation"
      tabindex="-1"
    >
      <div
        class="absolute bg-gray-800 border border-white/20 rounded-lg shadow-xl py-1 min-w-[180px] animate-fade-in"
        style="left: {subtitleContextMenu.x}px; top: {subtitleContextMenu.y}px;"
      >
        <button
          onclick={() => {
            if (subtitleContextMenu)
              playSubtitleFromList(subtitleContextMenu.sub);
            closeSubtitleContextMenu();
          }}
          class="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-indigo-500/20 hover:text-indigo-300 flex items-center gap-2 transition-colors"
        >
          <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"
            ><path d="M8 5v14l11-7z" /></svg
          >
          {t("sync.playSubtitle")}
        </button>
        <button
          onclick={() => {
            if (subtitleContextMenu)
              goToSubtitleManual(subtitleContextMenu.sub);
            closeSubtitleContextMenu();
          }}
          class="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-indigo-500/20 hover:text-indigo-300 flex items-center gap-2 transition-colors"
        >
          <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            ><path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 12a9 9 0 11-18 0 9 9 0 0118 0z"
            /></svg
          >
          {t("sync.goToSubtitle")}
        </button>
      </div>
    </div>
  {/if}

  {#if error}
    <div
      class="fixed bottom-4 right-4 glass-card bg-red-500/20 border border-red-500/30 text-white px-6 py-4 rounded-xl shadow-xl flex items-center gap-3 animate-fade-in"
    >
      <svg
        class="w-5 h-5 text-red-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        /></svg
      >
      <span class="text-red-200">{error}</span>
      <button
        onclick={() => (error = null)}
        class="text-red-400 hover:text-red-300 ml-2"
        aria-label="Close"
        ><svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          /></svg
        ></button
      >
    </div>
  {/if}

  {#if snackbarMessage}
    <Snackbar
      message={snackbarMessage}
      variant={snackbarVariant}
      onclose={() => (snackbarMessage = null)}
    />
  {/if}

  {#if showResetModal}
    <div
      class="absolute inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm"
    >
      <div
        class="p-6 max-w-sm w-full mx-4 shadow-2xl border border-white/10 rounded-2xl"
        style="background: #1e1e2e;"
      >
        <h3 class="text-lg font-semibold text-white mb-2">
          {t("sync.resetSync")}
        </h3>
        <p class="text-gray-400 text-sm mb-6">{t("sync.confirmReset")}</p>
        <div class="flex gap-3 justify-end">
          <button
            onclick={() => (showResetModal = false)}
            class="btn-secondary py-2 px-5 text-sm"
            >{t("sync.cancelReset")}</button
          >
          <button onclick={confirmReset} class="btn-danger py-2 px-5 text-sm"
            >OK</button
          >
        </div>
      </div>
    </div>
  {/if}

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "srt" ? "SRT Path" : "Media Path"}
    value={expandedPathField === "srt" ? status?.srt_path || "" : status?.video_path || ""}
    onclose={() => (expandedPathField = null)}
  />

  <InfoModal 
    section={helpSection} 
    sections={syncSections} 
    onclose={() => (helpSection = null)} 
  />
</div>

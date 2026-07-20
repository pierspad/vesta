<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { autoSyncStore, type AutoSyncProgressPayload } from "$lib/stores/autoSyncStore.svelte";
  import FooterActions from "$lib/components/FooterActions.svelte";
  import AutoSyncControls from "$lib/components/AutoSyncControls.svelte";
  import AutoSyncOverlay from "$lib/components/AutoSyncOverlay.svelte";
  import FilesPanel from "$lib/panels/FilesPanel.svelte";
  import SyncStatusPanel from "$lib/panels/SyncStatusPanel.svelte";
  import SubtitleListPanel from "$lib/panels/SubtitleListPanel.svelte";
  import WizardCheckpoint from "$lib/components/WizardCheckpoint.svelte";

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
  let audioSrc = $state<string | null>(null);
  let audioError = $state<string | null>(null);
  let hasAutoPaused = $state(false);
  let isPreparingMedia = $state(false);

  let showResetModal = $state(false);



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
  async function cancelAutoSync() {
    if (!autoSyncStore.isAutoSyncing || autoSyncStore.isCancelling) return;
    autoSyncStore.isCancelling = true;
    try {
      await invoke("sync_cancel_auto_sync");
      autoSyncStore.message = t("sync.autoSyncCancelling");
    } catch (e) {
      showSnackbar(`Cancel failed: ${e}`, "error");
      autoSyncStore.isCancelling = false;
    }
  }

  async function checkWhisperModels() {
    try {
      const models = await invoke<Array<{ id: string; downloaded: boolean }>>(
        "transcribe_list_models",
      );
      autoSyncStore.whisperModelsAvailable = models
        .filter((m) => m.downloaded)
        .map((m) => m.id);
    } catch {
      autoSyncStore.whisperModelsAvailable = [];
    }
  }

  async function startAutoSync(quick = false) {
    if (autoSyncStore.isAutoSyncing) return;
    if (!status?.is_loaded) {
      showSnackbar(t("sync.dropSrtFirst"), "warning");
      return;
    }
    if (!status?.video_path && !audioSrc) {
      showSnackbar(t("sync.needVideoForAnchor"), "warning");
      return;
    }

    await checkWhisperModels();
    if (autoSyncStore.whisperModelsAvailable.length === 0) {
      showSnackbar(
        t("transcribe.noBackendWarning"),
        "warning"
      );
      return;
    }

    // Prefer small > base > tiny > medium > large
    const preferredOrder = ["small", "base", "tiny", "medium", "large"];
    const modelId =
      preferredOrder.find((m) => autoSyncStore.whisperModelsAvailable.includes(m)) ??
      autoSyncStore.whisperModelsAvailable[0];

    autoSyncStore.isAutoSyncing = true;
    autoSyncStore.activeMode = quick ? "quick" : "precise";
    autoSyncStore.progress = 0;
    autoSyncStore.message = t("sync.autoSyncInProgress");
    addSyncLog(`Auto-sync started with model: ${modelId} (mode: ${quick ? 'quick' : 'precise'})`, "info");

    // Listen for progress events
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<AutoSyncProgressPayload>("sync-auto-progress", (event) => {
      autoSyncStore.progress = event.payload.percentage;
      autoSyncStore.message = autoSyncStore.resolveProgressMessage(event.payload);
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
        quick,
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
      autoSyncStore.isAutoSyncing = false;
      autoSyncStore.activeMode = null;
      autoSyncStore.isCancelling = false;
      autoSyncStore.progress = 0;
      autoSyncStore.message = "";
      unlisten();
    }
  }

  type SyncPanelId = "files" | "wizard" | "status" | "subtitleList";

  function syncDebug(message: string, payload?: Record<string, unknown>) {
    if (payload) {
      console.info(`[SyncTab] ${message}`, payload);
      return;
    }
    console.info(`[SyncTab] ${message}`);
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

  let mediaServerInfo: [number, string] | null = null;

  // Porta + token di sessione del media server locale (il token autentica
  // ogni richiesta /media: senza, il server risponde 403).
  async function getMediaServerInfo(): Promise<[number, string]> {
    if (mediaServerInfo) return mediaServerInfo;
    mediaServerInfo = await invoke<[number, string]>("get_media_server_info");
    return mediaServerInfo;
  }

  async function loadMediaFile(filePath: string): Promise<string> {
    // For non-browser-native formats (mkv, avi, etc.), transcode audio
    // via ffmpeg to OGG Opus for reliable WebKitGTK playback
    const needsTranscode = /\.(mkv|avi|mov|flv|ogm|vob|wma|m4b|m2ts|mpeg|mpg)$/i.test(filePath);
    if (needsTranscode) {
      isPreparingMedia = true;
      showSnackbar(t("sync.transcodingMedia"), "info");
    }
    try {
      const playbackPath = await invoke<string>("sync_prepare_media_for_playback", {
        path: filePath,
      });
      const [port, token] = await getMediaServerInfo();
      return `http://127.0.0.1:${port}/media?path=${encodeURIComponent(playbackPath)}&token=${token}`;
    } finally {
      if (needsTranscode) {
        isPreparingMedia = false;
      }
    }
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
    snackbar.show(message, variant, 3500);
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
      if (audioElement) {
        audioElement.pause();
      }
      return;
    }
    showSaveSuggestion = false;
    await goToCheckpoint(nextId);
  }

  async function goToCheckpoint(
    id: number,
    options: { updateList?: boolean; scrollList?: boolean; autoplay?: boolean } = {},
  ) {
    console.debug(`[SyncTab] goToCheckpoint(${id}) called, isNavigating=${isNavigating}`);
    if (isNavigating) return;
    const { updateList = true, scrollList = true, autoplay = true } = options;
    isNavigating = true;
    try {
      const sub = await invoke<SubtitleInfo>("sync_get_subtitle", { id });
      wizardSubtitle = sub;
      resetOffset();
      if (!wizardHistory.includes(id)) {
        wizardHistory = [...wizardHistory, id];
      }
      seekToSubtitleStart(sub);
      if (updateList) {
        await loadSubtitlesAround(id);
      }
      if (scrollList) {
        setTimeout(() => scrollToSubtitle(id), 50);
      } else if (subtitleListElement) {
        subtitleListElement.scrollTop = 0;
      }
      if (autoplay) {
        void safePlayAudio("goToCheckpoint");
      }
    } catch (e) {
      showSnackbar(`Error loading subtitle: ${e}`, "error");
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
        const nextId = computeNextCheckpoint();
        if (nextId !== null) {
          await goToCheckpoint(nextId, {
            updateList: false,
            scrollList: false,
            autoplay: false,
          });
        }
        await tryAutoSelectMediaForSrt(selectedPath);
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorLoadingSrt")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorLoadingAudio")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorLoadingSrt")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorAddingAnchor")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorAddingAnchor")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorRemovingAnchor")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorSaving")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorSaving")} ${e}`, "error");
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
        const nextId = computeNextCheckpoint();
        if (nextId !== null) {
          await goToCheckpoint(nextId, {
            updateList: false,
            scrollList: false,
            autoplay: false,
          });
        }
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorLoadingSrt")} ${e}`, "error");
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
      showSnackbar(`${t("sync.errorSaving")} ${e}`, "error");
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
          showSnackbar(`${t("sync.errorLoadingSrt")} ${e}`, "error");
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
          showSnackbar(`${t("sync.errorLoadingAudio")} ${e}`, "error");
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
      showSnackbar(`Undo failed: ${e}`, "error");
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (subtitleContextMenu) {
      const key = e.key.toLowerCase();
      if (key === "escape") {
        closeSubtitleContextMenu();
        e.preventDefault();
        return;
      }
      if (key === "p") {
        playSubtitleFromList(subtitleContextMenu.sub);
        closeSubtitleContextMenu();
        e.preventDefault();
        return;
      }
      if (key === "g") {
        goToSubtitleManual(subtitleContextMenu.sub);
        closeSubtitleContextMenu();
        e.preventDefault();
        return;
      }
    }

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
      if (hasAudio && !aiStore.killSwitchActive) startAutoSync();
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

    let activeListener = true;
    let unlistenDD: (() => void) | null = null;

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
        if (!activeListener) fn();
        else unlistenDD = fn;
      })
      .catch((e) => {
        console.warn("Failed to set up drag-drop listener:", e);
      });

    return () => {
      activeListener = false;
      syncDebug("unmount");
      window.removeEventListener("keydown", handleKeydown);
      if (unlistenDD) unlistenDD();
      if (singleClickTimer) clearTimeout(singleClickTimer);
    };
  });
</script>

<div
  class="h-full flex flex-col bg-gray-900 relative overflow-hidden"
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
  <video
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
  ></video>

  {#snippet panelContent(panelId: SyncPanelId)}
    {#if panelId === "files"}
      <FilesPanel
        srtPath={status?.srt_path ?? null}
        mediaPath={status?.video_path ?? null}
        srtLoaded={!!status?.is_loaded}
        onExpandSrt={() => (expandedPathField = "srt")}
        onExpandMedia={() => (expandedPathField = "media")}
        onBrowseSrt={selectSrtFile}
        onBrowseMedia={selectAudioFile}
      />
    {:else if panelId === "wizard"}
      <WizardCheckpoint
        srtLoaded={!!status?.is_loaded}
        totalSubtitles={status?.total_subtitles || 0}
        {showSaveSuggestion}
        {wizardSubtitle}
        {audioSrc}
        {audioError}
        {isPreparingMedia}
        bind:currentVideoTime
        {audioDuration}
        {audioElement}
        {isPlaying}
        {hasAudio}
        {offsetAdjustment}
        {formatTime}
        {formatOffset}
        onSaveFile={saveFile}
        onContinueChecking={() => (showSaveSuggestion = false)}
        onRetryAudio={() => {
          audioError = null;
          audioSrc = null;
          selectAudioFile();
        }}
        onTogglePlay={() =>
          hasAudio &&
          audioElement &&
          (isPlaying
            ? audioElement.pause()
            : void safePlayAudio("wizard-play-button"))}
        onReplay={replayCurrentSubtitle}
        onAdjustOffset={(delta) => {
          updateOffset(delta);
          replayCurrentSubtitle();
        }}
        onSkip={skipCheckpoint}
        onConfirm={confirmCurrentCheckpoint}
      />
    {:else if panelId === "status"}
      <SyncStatusPanel {status} {confidenceScore} {formatOffset} />
    {:else if panelId === "subtitleList"}
      <SubtitleListPanel
        {subtitles}
        isLoaded={!!status?.is_loaded}
        {currentPage}
        {totalPages}
        activeSubtitleId={wizardSubtitle?.id ?? null}
        {formatTime}
        {formatOffset}
        bind:listElement={subtitleListElement}
        onPageChange={loadPage}
        onClickSub={handleSubtitleClick}
        onDblClickSub={handleSubtitleDblClick}
        onContextMenu={openSubtitleContextMenu}
        onRemoveAnchor={removeAnchor}
      />
    {/if}
  {/snippet}

  <div class="flex-1 overflow-y-auto p-6 min-h-0">
    <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <div class="space-y-3 min-w-0 flex flex-col">
        {@render panelContent("files")}
        {@render panelContent("wizard")}
      </div>

      <div class="space-y-3 min-w-0 flex flex-col">
        {@render panelContent("status")}
        <div class="flex flex-col min-h-[520px] flex-1" role="region">
          {@render panelContent("subtitleList")}
        </div>
      </div>
    </div>
  </div>

  <!-- Fixed Bottom Band with Action Buttons -->
  <FooterActions>
    {#snippet left()}
      <div class="relative group">
        <button
          onclick={loadSession}
          class="px-5 py-2.5 bg-cyan-600 hover:bg-cyan-500 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-cyan-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] cursor-pointer"
        >
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
          </svg>
          {t("sync.loadSession")}
        </button>
        <div class="pointer-events-none absolute bottom-full left-0 z-50 mb-3 rounded-xl border border-cyan-500/30 bg-gray-950/95 p-3 text-center text-xs text-cyan-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("sync.tooltipLoadSession")}
        </div>
      </div>

      <div class="relative group">
        <button
          onclick={saveSession}
          disabled={!status?.is_loaded}
          class="px-5 py-2.5 bg-teal-600 hover:bg-teal-500 disabled:bg-teal-600/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-teal-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 disabled:cursor-not-allowed cursor-pointer"
        >
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
          </svg>
          {t("sync.saveSession")}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-teal-500/30 bg-gray-950/95 p-3 text-center text-xs text-teal-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("sync.tooltipSaveSession")}
        </div>
      </div>
    {/snippet}
    {#snippet center()}
      <AutoSyncControls
        canAutoSync={!!status?.is_loaded && hasAudio}
        onStart={() => startAutoSync(autoSyncStore.selectedMode === "quick")}
      />
    {/snippet}
    {#snippet right()}
      <!-- New Sync -->
      <div class="relative group">
        <button
          onclick={() => (showResetModal = true)}
          disabled={!(status?.is_loaded || audioSrc)}
          class="px-5 py-2.5 bg-amber-500 hover:bg-amber-400 disabled:bg-amber-500/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-amber-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 disabled:cursor-not-allowed cursor-pointer"
        >
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          {t("sync.newSync")}
        </button>
        <div class="pointer-events-none absolute bottom-full left-1/2 z-50 mb-3 -translate-x-1/2 rounded-xl border border-amber-500/30 bg-gray-950/95 p-3 text-center text-xs text-amber-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("sync.newSyncDesc")}
        </div>
      </div>

      <!-- Save File -->
      <div class="relative group">
        <button
          onclick={saveFile}
          disabled={!status?.is_loaded}
          class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-500 disabled:bg-emerald-600/55 text-white rounded-xl font-bold text-sm transition-all shadow-lg shadow-emerald-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 disabled:cursor-not-allowed cursor-pointer"
        >
          <svg
            class="w-4 h-4 text-white"
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
          {t("sync.saveFile")}
        </button>
        <div class="pointer-events-none absolute bottom-full right-0 z-50 mb-3 rounded-xl border border-green-500/30 bg-gray-950/95 p-3 text-center text-xs text-green-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-normal w-72">
          {t("sync.tooltipSaveFile")}
        </div>
      </div>
    {/snippet}
  </FooterActions>

  <AutoSyncOverlay onCancel={cancelAutoSync} />

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
        class="vesta-context-menu animate-fade-in"
        style="left: {subtitleContextMenu.x}px; top: {subtitleContextMenu.y}px;"
      >
        <button
          onclick={() => {
            if (subtitleContextMenu)
              playSubtitleFromList(subtitleContextMenu.sub);
            closeSubtitleContextMenu();
          }}
          class="vesta-context-menu-item"
        >
          <span class="inline-flex items-center gap-2">
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"
              ><path d="M8 5v14l11-7z" /></svg
            >
            {t("sync.playSubtitle")}
          </span>
          <kbd>P</kbd>
        </button>
        <button
          onclick={() => {
            if (subtitleContextMenu)
              goToSubtitleManual(subtitleContextMenu.sub);
            closeSubtitleContextMenu();
          }}
          class="vesta-context-menu-item"
        >
          <span class="inline-flex items-center gap-2">
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
          </span>
          <kbd>G</kbd>
        </button>
      </div>
    </div>
  {/if}




  <ConfirmDialog
    show={showResetModal}
    title={t("sync.resetSync") || "Ripristinare sessione?"}
    message={t("sync.confirmReset") || "Tutti i dati correnti di sincronizzazione andranno persi."}
    confirmText="OK"
    cancelText={t("sync.cancelReset") || "Annulla"}
    variant="danger"
    on:cancel={() => (showResetModal = false)}
    on:confirm={confirmReset}
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "srt" ? "SRT Path" : "Media Path"}
    value={expandedPathField === "srt" ? status?.srt_path || "" : status?.video_path || ""}
    onclose={() => (expandedPathField = null)}
  />
</div>

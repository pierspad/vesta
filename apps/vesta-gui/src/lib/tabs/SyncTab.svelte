<script lang="ts">
  import { anchorOffsetSpread } from "$lib/workflows/syncAssessment";
  import { SyncOffsetAdjustment } from "$lib/workflows/syncOffsetAdjustment.svelte";
  import WorkflowWorkspace from "$lib/components/WorkflowWorkspace.svelte";
  import { localWhisperOperation } from "$lib/utils/asyncControl.svelte";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import PathPreviewModal from "$lib/modals/PathPreviewModal.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { createSnackbarNotifier } from "$lib/stores/snackbarStore.svelte";
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { autoSyncStore } from "$lib/stores/autoSyncStore.svelte";
  import AutoSyncOverlay from "$lib/components/AutoSyncOverlay.svelte";
  import FilesPanel from "$lib/panels/FilesPanel.svelte";
  import SyncStatusPanel from "$lib/panels/SyncStatusPanel.svelte";
  import SubtitleListPanel from "$lib/panels/SubtitleListPanel.svelte";
  import type { TranscribeTierEntry } from "$lib/config/transcribeTiers";
  import WizardCheckpoint from "$lib/components/WizardCheckpoint.svelte";
  import { formatSyncMediaError } from "$lib/utils/syncMediaError";
  import { errorMessage } from "$lib/utils/errorMessage";
  import { whisperModelsStore as modelStore } from "$lib/stores/whisperModelsStore.svelte";
  import {
    type SubtitleInfo,
    type SyncStatus,
    type AnchorInfo,
    formatSyncTime as formatTime,
    formatSyncOffset as formatOffset,
    computeNextSyncCheckpoint,
    SYNC_MEDIA_EXTENSIONS,
    isSyncSrtFile,
    isSyncMediaFile,
  } from "./sync/syncFormatters";
  import { executeAutoSync } from "./sync/syncAutoRunner";
  import { handleSyncKeydown } from "./sync/syncKeyboard";
  import { prepareSyncMediaUrl, playSyncMediaElement } from "./sync/syncMediaPlayback";
  import SyncFooterBar from "./sync/SyncFooterBar.svelte";
  import SyncSubtitleContextMenu from "./sync/SyncSubtitleContextMenu.svelte";

  interface Props {
    active?: boolean;
  }

  let { active = true }: Props = $props();

  let t = $derived($locale);
  let selectedEngine = $state<TranscribeTierEntry | null | undefined>(undefined);

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



  let offsetSpread = $derived(anchorOffsetSpread(anchors));

  let wizardSubtitle = $state<SubtitleInfo | null>(null);
  const offset = new SyncOffsetAdjustment();
  let offsetAdjustment = $derived(offset.value);
  const updateOffset = (delta: number) => offset.update(delta);
  const resetOffset = () => offset.reset();

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

  let manualAnchors = $derived(anchors.filter((a) => a.is_manual));

  // ─── Auto-Sync with Whisper ────────────────────────────
  async function cancelAutoSync() {
    if (!autoSyncStore.isAutoSyncing || autoSyncStore.isCancelling) return;
    try {
      if (!await autoSyncStore.operation.cancel(() => invoke("sync_cancel_auto_sync"))) return;
      autoSyncStore.message = t("sync.autoSyncCancelling");
    } catch (e) {
      showSnackbar(`${t("sync.cancelFailed")} — ${errorMessage(e)}`, "error");
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
    if (autoSyncStore.isAutoSyncing || aiStore.killSwitchActive) return;
    if (!status?.is_loaded) {
      showSnackbar(t("sync.dropSrtFirst"), "warning");
      return;
    }
    if (!status?.video_path && !audioSrc) {
      showSnackbar(t("sync.needVideoForAnchor"), "warning");
      return;
    }

    await executeAutoSync({
      quick,
      entry: selectedEngine,
      checkWhisperModels,
      onStatus: (nextStatus) => { status = nextStatus; },
      refreshAfterSync: async () => {
        await refreshCurrentSubtitles();
        await loadAnchors();
      },
      showSnackbar,
      t,
    });
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
      showSnackbar(`${t("sync.inputMediaFile")} · ${getFileName(suggestedPath)}`, "success");
    } catch (e) {
      syncDebug("auto-media-suggestion-failed", { error: String(e) });
    }
  }

  async function safePlayAudio(source: string): Promise<boolean> {
    if (!active) return false;
    return playSyncMediaElement(audioElement, {
      source,
      hasAudio,
      audioError,
      isStartingPlayback,
      setStartingPlayback: (value) => { isStartingPlayback = value; },
      debug: syncDebug,
      showSnackbar,
      t,
    });
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

  async function loadMediaFile(filePath: string): Promise<string> {
    return prepareSyncMediaUrl(filePath, {
      setPreparing: (value) => { isPreparingMedia = value; },
      setAudioError: (message) => { audioError = message; },
      showSnackbar,
      t,
    });
  }

  function cleanupAudioSrc() {
    audioSrc = null;
  }

  const showSnackbar = createSnackbarNotifier();

  function computeNextCheckpoint(): number | null {
    return computeNextSyncCheckpoint(status, wizardHistory, manualAnchors);
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
      if (!active) return;
      wizardSubtitle = sub;
      resetOffset();
      if (!wizardHistory.includes(id)) {
        wizardHistory = [...wizardHistory, id];
      }
      seekToSubtitleStart(sub);
      if (updateList) {
        await loadSubtitlesAround(id);
        if (!active) return;
      }
      if (scrollList) {
        setTimeout(() => {
          if (active) scrollToSubtitle(id);
        }, 50);
      } else if (subtitleListElement) {
        subtitleListElement.closest(".glass-card")?.scrollIntoView({ block: "start" });
      }
      if (autoplay && active) {
        void safePlayAudio("goToCheckpoint");
      }
    } catch (e) {
      showSnackbar(`${t("sync.subtitleLoadFailed")} — ${errorMessage(e)}`, "error");
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
        filters: [{ name: t("sync.inputSrtFile"), extensions: ["srt"] }],
      });
      if (selected) {
        const selectedPath = selected as string;
        status = await invoke<SyncStatus>("sync_load_srt", {
          path: selectedPath,
        });
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
      showSnackbar(`${t("sync.errorLoadingSrt")} ${errorMessage(e)}`, "error");
    }
  }

  async function selectAudioFile() {
    try {
      const selected = await guardedOpen({
        multiple: false,
        filters: [
          {
            name: t("flashcards.mediaFiles"),
            extensions: SYNC_MEDIA_EXTENSIONS,
          },
        ],
      });
      if (selected) {
        const path = selected as string;
        audioError = null;
        cleanupAudioSrc();
        audioSrc = await loadMediaFile(path);
        status = await invoke<SyncStatus>("sync_set_video", { path });
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorLoadingAudio")} ${errorMessage(e)}`, "error");
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
        subtitleListElement.closest(".glass-card")?.scrollIntoView({ block: "start" });
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorLoadingSrt")} ${errorMessage(e)}`, "error");
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
      showSnackbar(`${t("sync.errorAddingAnchor")} ${errorMessage(e)}`, "error");
    }
  }

  async function refreshCurrentSubtitles() {
    console.debug(`[SyncTab] refreshCurrentSubtitles (page ${currentPage})`);
    await loadPage(currentPage);
  }

  async function confirmCurrentCheckpoint() {
    console.debug("[SyncTab] confirmCurrentCheckpoint called");
    if (syncEditBusy) {
      syncDebug("confirmCurrentCheckpoint ignored (already running)");
      return;
    }
    if (!status?.is_loaded || showResetModal || !wizardSubtitle) return;
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
    } catch (e) {
      showSnackbar(`${t("sync.errorAddingAnchor")} ${errorMessage(e)}`, "error");
      syncDebug("confirmCurrentCheckpoint failed", { error: String(e) });
    } finally {
      isConfirmingCheckpoint = false;
    }
  }

  async function removeAnchor(subtitleId: number) {
    if (syncEditBusy) return;
    isChangingHistory = true;
    try {
      status = await invoke<SyncStatus>("sync_remove_anchor", { subtitleId });
      await refreshCurrentSubtitles();
      await loadAnchors();
      wizardHistory = wizardHistory.filter(id => id !== subtitleId);
      showSaveSuggestion = false;
      await goToCheckpoint(subtitleId, { autoplay: false });
    } catch (e) {
      showSnackbar(`${t("sync.errorRemovingAnchor")} ${errorMessage(e)}`, "error");
    } finally { isChangingHistory = false; }
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
      showSnackbar(t("sync.subtitleNotFound", { id }), "warning");
    }
  }

  async function saveFile() {
    console.debug("[SyncTab] saveFile");
    try {
      const selected = await guardedSave({
        filters: [{ name: t("sync.inputSrtFile"), extensions: ["srt"] }],
        defaultPath: status?.srt_path?.replace(".srt", ".synced.srt"),
      });
      if (selected) {
        await invoke<string>("sync_save_file", { outputPath: selected });
        showSnackbar(`${t("sync.fileSaved")} ${selected}`, "success");
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorSaving")} ${errorMessage(e)}`, "error");
    }
  }

  async function saveSession() {
    console.debug("[SyncTab] saveSession");
    try {
      const selected = await guardedSave({
        filters: [{ name: t("sync.sessionFiles"), extensions: ["json"] }],
      });
      if (selected) {
        await invoke<string>("sync_save_session", { sessionPath: selected });
        showSnackbar(`${t("sync.sessionSaved")} ${selected}`, "success");
      }
    } catch (e) {
      showSnackbar(`${t("sync.errorSaving")} ${errorMessage(e)}`, "error");
    }
  }

  async function loadSession() {
    console.debug("[SyncTab] loadSession");
    try {
      const selected = await guardedOpen({
        filters: [{ name: t("sync.sessionFiles"), extensions: ["json"] }],
      });
      if (selected) {
        status = await invoke<SyncStatus>("sync_load_session", {
          sessionPath: selected as string,
        });
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
      showSnackbar(`${t("sync.errorLoadingSrt")} ${errorMessage(e)}`, "error");
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
    } catch (e) {
      showSnackbar(`${t("sync.errorSaving")} ${errorMessage(e)}`, "error");
    }
  }

  async function handleDroppedPaths(paths: string[]) {
    if (paths.length === 0) return;

    // Sort paths so SRT files are processed first
    const sortedPaths = [...paths].sort((a, b) => {
      const isASrt = isSyncSrtFile(a);
      const isBSrt = isSyncSrtFile(b);
      if (isASrt && !isBSrt) return -1;
      if (!isASrt && isBSrt) return 1;
      return 0;
    });

    for (const filePath of sortedPaths) {
      const fileName = getFileName(filePath);
      if (isSyncSrtFile(fileName)) {
        try {
          status = await invoke<SyncStatus>("sync_load_srt", {
            path: filePath,
          });
          await loadSubtitles();
          await loadAnchors();
          wizardHistory = [];
          showSaveSuggestion = false;
          await advanceWizard();
          await tryAutoSelectMediaForSrt(filePath);
        } catch (e: any) {
          showSnackbar(`${t("sync.errorLoadingSrt")} ${errorMessage(e)}`, "error");
        }
      } else if (isSyncMediaFile(fileName)) {
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
        } catch (e: any) {
          showSnackbar(`${t("sync.errorLoadingAudio")} ${errorMessage(e)}`, "error");
        }
      }
    }
  }

  let isChangingHistory = $state(false);
  let syncEditBusy = $derived(isChangingHistory || isConfirmingCheckpoint || autoSyncStore.isAutoSyncing);

  async function changeSyncHistory(direction: "undo" | "redo") {
    if (syncEditBusy || !status?.[direction === "undo" ? "can_undo" : "can_redo"]) return;
    isChangingHistory = true;
    audioElement?.pause();
    closeSubtitleContextMenu();
    if (singleClickTimer) { clearTimeout(singleClickTimer); singleClickTimer = null; }
    const previous = anchors;
    try {
      status = await invoke<SyncStatus>(direction === "undo" ? "sync_undo" : "sync_redo");
      await loadAnchors();
      await refreshCurrentSubtitles();
      wizardHistory = anchors.filter(anchor => anchor.is_manual).map(anchor => anchor.subtitle_id);
      showSaveSuggestion = false;
      const changed = [...previous, ...anchors].find(anchor => {
        const old = previous.find(a => a.subtitle_id === anchor.subtitle_id);
        const next = anchors.find(a => a.subtitle_id === anchor.subtitle_id);
        return old?.corrected_time_ms !== next?.corrected_time_ms || old?.is_manual !== next?.is_manual;
      });
      if (changed) await goToCheckpoint(changed.subtitle_id, { autoplay: false });
      else if (wizardSubtitle) await goToCheckpoint(wizardSubtitle.id, { autoplay: false });
    } catch (error) {
      showSnackbar(errorMessage(error), "error");
    } finally { isChangingHistory = false; }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!active) return;
    handleSyncKeydown(e, {
      active,
      showResetModal,
      expandedPathField,
      subtitleContextMenu,
      canStartAutoSync:
        !!selectedEngine &&
        !!status?.is_loaded &&
        hasAudio &&
        !localWhisperOperation.busy &&
        !aiStore.killSwitchActive &&
        !syncEditBusy,
      canReset: !syncEditBusy,
      syncEditBusy,
      hasAudio,
      isLoaded: !!status?.is_loaded,
      hasMediaSrc: !!audioSrc,
      isPlaying,
      wizardSubtitleId: wizardSubtitle?.id ?? null,
      totalSubtitles: status?.total_subtitles ?? 0,
      anchorIds: anchors.map((anchor) => anchor.subtitle_id),
      onCloseContextMenu: closeSubtitleContextMenu,
      onPlaySub: (sub) => { void playSubtitleFromList(sub); },
      onGoToSub: (sub) => { void goToSubtitleManual(sub); },
      onHistory: (direction) => { void changeSyncHistory(direction); },
      onSelectSrt: () => { void selectSrtFile(); },
      onStartAutoSync: () => { void startAutoSync(autoSyncStore.selectedMode === "quick"); },
      onRequestReset: () => { showResetModal = true; },
      onLoadSession: () => { void loadSession(); },
      onSaveSession: () => { void saveSession(); },
      onSaveFile: () => { void saveFile(); },
      onTogglePlay: () => {
        if (!audioElement) return;
        if (isPlaying) audioElement.pause();
        else void safePlayAudio("keyboard-play");
      },
      onReplaySubtitle: replayCurrentSubtitle,
      onSeekBy: (seconds) => {
        if (audioElement) audioElement.currentTime += seconds;
      },
      onAdjustOffset: (deltaMs) => {
        if (!wizardSubtitle) return;
        updateOffset(deltaMs);
        seekToSubtitleStart(wizardSubtitle);
      },
      onConfirmAnchor: () => { void confirmCurrentCheckpoint(); },
      onGoToSuggested: () => { void advanceWizard(); },
      onGoToCheckpoint: (id) => {
        showSaveSuggestion = false;
        void goToCheckpoint(id);
      },
    });
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

  $effect(() => {
    if (!active && audioElement && !audioElement.paused) {
      audioElement.pause();
    }
  });

  onMount(() => {
    syncDebug("mount", { active });
    window.addEventListener("keydown", handleKeydown);

    void modelStore.refreshModels();
    void checkWhisperModels();

    const onWhisperUpdated = () => {
      void checkWhisperModels();
    };
    window.addEventListener("whisper-model-updated", onWhisperUpdated);
    window.addEventListener("vesta-whisper-models-updated", onWhisperUpdated);

    const cleanupDragDrop = setupWebviewDragDrop({
      isActive: () => active,
      setDraggingOver: (v) => (isDraggingOver = v),
      onDrop: (paths) => {
        syncDebug("file-drop", { count: paths.length });
        handleDroppedPaths(paths);
      },
    });

    return () => {
      cleanupDragDrop();
      offset.dispose();
      syncDebug("unmount");
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("whisper-model-updated", onWhisperUpdated);
      window.removeEventListener("vesta-whisper-models-updated", onWhisperUpdated);
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
      const code = mediaErr?.code || 0;
      const msg = mediaErr?.message || "";
      audioError = formatSyncMediaError(code, msg, t);
      syncDebug("audio:onerror", {
        code,
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
        canUndo={!!status?.can_undo && !syncEditBusy}
        canRedo={!!status?.can_redo && !syncEditBusy}
        onHistory={changeSyncHistory}
        onSkip={skipCheckpoint}
        onConfirm={confirmCurrentCheckpoint}
      />
    {:else if panelId === "status"}
      <SyncStatusPanel {status} {offsetSpread} {formatOffset} />
    {:else if panelId === "subtitleList"}
      <SubtitleListPanel
        pageSize={PAGE_SIZE}
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
        editDisabled={syncEditBusy}
      />
    {/if}
  {/snippet}

  <div class="flex flex-1 flex-col overflow-hidden p-6 min-h-0">
    <WorkflowWorkspace>
      {#snippet files()}{@render panelContent("files")}{/snippet}
      <div class="grid grid-cols-1 items-start gap-6 xl:grid-cols-2">
        <div class="flex min-w-0 flex-col gap-3">
          {@render panelContent("status")}
          {@render panelContent("wizard")}
        </div>
        <div class="flex min-h-0 min-w-0 flex-col xl:relative xl:self-stretch" role="region">
          <div class="flex max-h-[42rem] min-h-0 flex-col xl:absolute xl:inset-0 xl:max-h-none">
            {@render panelContent("subtitleList")}
          </div>
        </div>
      </div>
    </WorkflowWorkspace>
  </div>

  <SyncFooterBar
    {active}
    canReset={!!(status?.is_loaded || audioSrc)}
    canSave={!!status?.is_loaded}
    canAutoSync={!!selectedEngine && !!status?.is_loaded && hasAudio && !localWhisperOperation.busy && !aiStore.killSwitchActive}
    bind:selectedEngine
    onLoadSession={loadSession}
    onStartAutoSync={() => startAutoSync(autoSyncStore.selectedMode === "quick")}
    onRequestReset={() => (showResetModal = true)}
    onSaveFile={saveFile}
  />

  <AutoSyncOverlay onCancel={cancelAutoSync} />

  {#if subtitleContextMenu}
    <SyncSubtitleContextMenu
      contextMenu={subtitleContextMenu}
      {syncEditBusy}
      onClose={closeSubtitleContextMenu}
      onRemoveAnchor={(id) => { void removeAnchor(id); }}
      onPlay={(sub) => { void playSubtitleFromList(sub); }}
      onGoTo={(sub) => { void goToSubtitleManual(sub); }}
    />
  {/if}




  <ConfirmDialog
    show={showResetModal}
    title={t("sync.resetSync")}
    message={t("sync.confirmReset")}
    confirmText="OK"
    cancelText={t("sync.cancelReset")}
    variant="danger"
    on:cancel={() => (showResetModal = false)}
    on:confirm={confirmReset}
  />

  <PathPreviewModal
    isOpen={!!expandedPathField}
    title={expandedPathField === "srt" ? t("sync.inputSrtFile") : t("sync.inputMediaFile")}
    value={expandedPathField === "srt" ? status?.srt_path || "" : status?.video_path || ""}
    onclose={() => (expandedPathField = null)}
  />
</div>

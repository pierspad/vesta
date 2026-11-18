import { historyShortcutAction, localShortcutAction } from "$lib/utils/shortcuts";
import type { SubtitleInfo } from "./syncFormatters";

export interface SyncKeyboardContext {
  active: boolean;
  showResetModal: boolean;
  expandedPathField: "srt" | "media" | null;
  subtitleContextMenu: { sub: SubtitleInfo } | null;
  canStartAutoSync: boolean;
  canReset: boolean;
  syncEditBusy: boolean;
  hasAudio: boolean;
  isLoaded: boolean;
  hasMediaSrc: boolean;
  isPlaying: boolean;
  wizardSubtitleId: number | null;
  totalSubtitles: number;
  anchorIds: number[];
  onCloseContextMenu: () => void;
  onPlaySub: (sub: SubtitleInfo) => void;
  onGoToSub: (sub: SubtitleInfo) => void;
  onHistory: (direction: "undo" | "redo") => void;
  onSelectSrt: () => void;
  onStartAutoSync: () => void;
  onRequestReset: () => void;
  onLoadSession: () => void;
  onSaveSession: () => void;
  onSaveFile: () => void;
  onTogglePlay: () => void;
  onReplaySubtitle: () => void;
  onSeekBy: (seconds: number) => void;
  onAdjustOffset: (deltaMs: number) => void;
  onConfirmAnchor: () => void;
  onGoToSuggested: () => void;
  onGoToCheckpoint: (id: number) => void;
}

/** Dispatch global keyboard shortcuts scoped to the active Sync tab. */
export function handleSyncKeydown(e: KeyboardEvent, ctx: SyncKeyboardContext): void {
  if (!ctx.active || ctx.showResetModal || ctx.expandedPathField || e.defaultPrevented || e.isComposing) {
    return;
  }
  const target = e.target as HTMLElement | null;
  if (target?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="dialog"]')) {
    return;
  }
  // Let Enter/Space activate the focused control natively while preserving P/G in the context menu.
  if ((e.key === "Enter" || e.key === " ") && target?.closest("button, a")) return;

  if (ctx.subtitleContextMenu) {
    const key = e.key.toLowerCase();
    if (key === "escape") {
      ctx.onCloseContextMenu();
      e.preventDefault();
      return;
    }
    if (key === "p") {
      ctx.onPlaySub(ctx.subtitleContextMenu.sub);
      ctx.onCloseContextMenu();
      e.preventDefault();
      return;
    }
    if (key === "g") {
      ctx.onGoToSub(ctx.subtitleContextMenu.sub);
      ctx.onCloseContextMenu();
      e.preventDefault();
      return;
    }
  }

  if (
    document.activeElement?.tagName === "INPUT" ||
    document.activeElement?.tagName === "TEXTAREA"
  ) {
    return;
  }

  const historyAction = historyShortcutAction(e, "sync");
  if (historyAction) {
    e.preventDefault();
    ctx.onHistory(historyAction);
    return;
  }

  const action = localShortcutAction(e, "sync");
  if (!action) return;
  e.preventDefault();
  switch (action) {
    case "openSrt":
      ctx.onSelectSrt();
      break;
    case "startAutoSync":
      if (ctx.canStartAutoSync) ctx.onStartAutoSync();
      break;
    case "newSync":
      if ((ctx.isLoaded || ctx.hasMediaSrc) && ctx.canReset) ctx.onRequestReset();
      break;
    case "loadSession":
      if (!ctx.syncEditBusy) ctx.onLoadSession();
      break;
    case "saveSession":
      if (ctx.hasAudio && !ctx.syncEditBusy) ctx.onSaveSession();
      break;
    case "saveFile":
      if (ctx.isLoaded && !ctx.syncEditBusy) ctx.onSaveFile();
      break;
    case "playPause":
      if (ctx.hasAudio) ctx.onTogglePlay();
      break;
    case "replaySubtitle":
      ctx.onReplaySubtitle();
      break;
    case "seekBack":
    case "seekForward":
    case "seekBackFast":
    case "seekForwardFast":
      if (ctx.hasAudio) {
        const distance = action.endsWith("Fast") ? 1 : 0.1;
        ctx.onSeekBy(action.startsWith("seekBack") ? -distance : distance);
      }
      break;
    case "offsetUp":
    case "offsetDown":
    case "offsetUpFast":
    case "offsetDownFast":
      if (ctx.wizardSubtitleId !== null && ctx.hasAudio && !ctx.syncEditBusy) {
        const amount = action.endsWith("Fast") ? 500 : 100;
        ctx.onAdjustOffset(action.startsWith("offsetDown") ? -amount : amount);
      }
      break;
    case "confirmAnchor":
      ctx.onConfirmAnchor();
      break;
    case "goToSuggested":
      if (!ctx.syncEditBusy) ctx.onGoToSuggested();
      break;
    case "prevSubtitle":
    case "nextSubtitle":
      if (ctx.wizardSubtitleId !== null && ctx.totalSubtitles > 0 && !ctx.syncEditBusy) {
        const id = ctx.wizardSubtitleId + (action === "nextSubtitle" ? 1 : -1);
        if (id >= 1 && id <= ctx.totalSubtitles) ctx.onGoToCheckpoint(id);
      }
      break;
    case "prevAnchor":
    case "nextAnchor": {
      if (ctx.wizardSubtitleId === null || ctx.syncEditBusy) break;
      const ids = [...ctx.anchorIds].sort((a, b) => a - b);
      const current = ctx.wizardSubtitleId;
      const id = action === "nextAnchor"
        ? ids.find((item) => item > current)
        : ids.findLast((item) => item < current);
      if (id !== undefined) ctx.onGoToCheckpoint(id);
      break;
    }
  }
}

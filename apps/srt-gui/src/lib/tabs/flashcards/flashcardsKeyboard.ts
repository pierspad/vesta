import { localShortcutAction } from "$lib/utils/shortcuts";
import { isInputActive } from "$lib/utils/keyboardFocus";

export interface FlashcardsKeyboardContext {
  active: boolean;
  bottomContextMenuOpen: boolean;
  episodeContextMenuIdx: number | null;
  episodeHasMedia: boolean;
  previewVisible: boolean;
  canRunFlashcards: boolean;
  isBusy: boolean;
  isPreviewLoading: boolean;
  onCloseBottomMenu: () => void;
  onCloseEpisodeMenu: () => void;
  onEditContextEpisode: () => void;
  onOpenEpisodeMediaSettings: (idx: number) => void;
  onRemoveEpisode: (idx: number) => void;
  onStartGeneration: () => void;
  onCancelGeneration: () => void;
  onLoadPreview: () => void;
}

/** Dispatch global keyboard shortcuts scoped to the active Flashcards tab. */
export function handleFlashcardsKeydown(e: KeyboardEvent, ctx: FlashcardsKeyboardContext): void {
  if (!ctx.active || e.defaultPrevented || e.isComposing) return;
  if (ctx.bottomContextMenuOpen && e.key.toLowerCase() === "escape") {
    ctx.onCloseBottomMenu();
    e.preventDefault();
    return;
  }
  if (ctx.episodeContextMenuIdx !== null) {
    if (
      document.activeElement?.tagName === "INPUT" ||
      document.activeElement?.tagName === "TEXTAREA"
    ) {
      return;
    }

    const key = e.key.toLowerCase();
    if (key === "escape") {
      ctx.onCloseEpisodeMenu();
      e.preventDefault();
      return;
    }
    if (key === "e") {
      ctx.onEditContextEpisode();
      ctx.onCloseEpisodeMenu();
      e.preventDefault();
      return;
    }
    if (key === "s") {
      if (ctx.episodeHasMedia) {
        ctx.onOpenEpisodeMediaSettings(ctx.episodeContextMenuIdx);
      }
      ctx.onCloseEpisodeMenu();
      e.preventDefault();
      return;
    }
    if (key === "d" || key === "delete") {
      ctx.onRemoveEpisode(ctx.episodeContextMenuIdx);
      ctx.onCloseEpisodeMenu();
      e.preventDefault();
      return;
    }
  }
  if (ctx.episodeContextMenuIdx !== null || ctx.bottomContextMenuOpen || ctx.previewVisible || isInputActive()) {
    return;
  }
  const action = localShortcutAction(e, "flashcards");
  if (action === "generateFlashcards" && ctx.canRunFlashcards && !ctx.isBusy && !ctx.isPreviewLoading) {
    e.preventDefault();
    ctx.onStartGeneration();
  } else if (action === "cancelGeneration" && ctx.isBusy) {
    e.preventDefault();
    ctx.onCancelGeneration();
  } else if (action === "previewCards" && ctx.canRunFlashcards && !ctx.isBusy && !ctx.isPreviewLoading) {
    e.preventDefault();
    ctx.onLoadPreview();
  }
}

import { LatestRequest } from "$lib/utils/asyncControl.svelte";
import { t } from "$lib/i18n";
import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { snackbar } from "$lib/stores/snackbarStore.svelte";

export type PreviewFilter = "all" | "active" | "inactive";

export interface PreviewLine {
  index: number;
  start_ms: number;
  end_ms: number;
  subs1_text: string;
  subs2_text: string | null;
  active: boolean;
  leading_context: unknown[];
  trailing_context: unknown[];
  [key: string]: unknown;
}

const PER_PAGE = 50;
const UNDO_LIMIT = 50;

const TRANSCODE_EXTENSIONS = /\.(mkv|avi|mov|flv|ogm|vob|wma|m4b|m2ts|mpeg|mpg)$/i;

/**
 * Owns the "Preview" dialog of the flashcards flow: the fetched subtitle
 * lines, the table's filter/search/pagination, the in-dialog media player,
 * and the right-click context menu. Extracted out of FlashcardsTab.svelte so
 * the dialog's state machine can be reasoned about (and rendered) on its own;
 * see FlashcardsPreviewModal.svelte for the view.
 */
export class PreviewStore {
  private loadRequests = new LatestRequest();
  private playbackRequests = new LatestRequest();
  visible = $state(false);
  lines = $state<PreviewLine[]>([]);
  loading = $state(false);
  filter = $state<PreviewFilter>("all");
  search = $state("");
  page = $state(1);

  /** Subtitle paths the currently loaded `lines` were fetched for — lets
   * generation reuse in-dialog edits only when the form still matches. */
  previewedSubsPath = $state<string | null>(null);
  previewedNativeSubsPath = $state<string | null>(null);

  contextMenuVisible = $state(false);
  contextMenuX = $state(0);
  contextMenuY = $state(0);
  contextMenuLine = $state<PreviewLine | null>(null);

  playingLine = $state<PreviewLine | null>(null);
  isPlaying = $state(false);
  playerElement = $state<HTMLMediaElement | null>(null);

  private undoStack = $state<PreviewLine[][]>([]);
  private mediaServerInfo: [number, string] | null = null;

  filtered = $derived(
    this.lines.filter((line) => {
      const matchesFilter =
        this.filter === "all" ||
        (this.filter === "active" && line.active) ||
        (this.filter === "inactive" && !line.active);
      const needle = this.search.toLowerCase();
      const matchesSearch =
        !needle ||
        line.subs1_text.toLowerCase().includes(needle) ||
        (line.subs2_text?.toLowerCase().includes(needle) ?? false);
      return matchesFilter && matchesSearch;
    }),
  );

  totalPages = $derived(Math.max(1, Math.ceil(this.filtered.length / PER_PAGE)));

  paged = $derived(this.filtered.slice((this.page - 1) * PER_PAGE, this.page * PER_PAGE));

  activeCardNumbers = $derived.by(() => {
    const numbers = new Map<number, number>();
    let count = 0;
    for (const line of this.lines) {
      if (line.active) numbers.set(line.index, ++count);
    }
    return numbers;
  });

  /** Fetches preview lines for `config` and opens the dialog. Throws on
   * failure so the caller can surface its own error message. */
  async load(config: Record<string, unknown>, onLoaded?: (lines: PreviewLine[]) => void) {
    const request = this.loadRequests.begin();
    this.stopPlayback();
    this.loading = true;
    this.visible = true;
    try {
      const lines = await invoke<PreviewLine[]>("flashcard_preview", { config });
      if (!request.isCurrent()) return;
      this.lines = lines;
      this.previewedSubsPath = config.target_subs_path as string;
      this.previewedNativeSubsPath = (config.native_subs_path as string | null) ?? null;
      this.undoStack = [];
      onLoaded?.(this.lines);
    } catch (error) {
      if (!request.isCurrent()) return;
      this.visible = false;
      throw error;
    } finally {
      if (request.isCurrent()) this.loading = false;
    }
  }

  close() {
    this.loadRequests.invalidate();
    this.loading = false;
    this.visible = false;
    this.contextMenuVisible = false;
    this.stopPlayback();
  }

  resetPaging(expertMode: boolean) {
    this.page = 1;
    if (!expertMode) this.filter = "all";
  }

  private pushUndo() {
    if (this.undoStack.length >= UNDO_LIMIT) this.undoStack.shift();
    this.undoStack.push(
      this.lines.map((line) => ({
        ...line,
        leading_context: [...line.leading_context],
        trailing_context: [...line.trailing_context],
      })),
    );
  }

  undo() {
    const previous = this.undoStack.pop();
    if (previous) this.lines = previous;
  }

  toggleLineActive(line: PreviewLine) {
    this.pushUndo();
    line.active = !line.active;
  }

  openContextMenu(e: MouseEvent, line: PreviewLine) {
    this.contextMenuLine = line;
    this.contextMenuX = e.clientX;
    this.contextMenuY = e.clientY;
    this.contextMenuVisible = true;
  }

  closeContextMenu() {
    this.contextMenuVisible = false;
  }

  private async getMediaServerInfo(): Promise<[number, string]> {
    if (!this.mediaServerInfo) {
      this.mediaServerInfo = await invoke<[number, string]>("get_media_server_info");
    }
    return this.mediaServerInfo;
  }

  async playLine(line: PreviewLine, mediaPath: string) {
    if (!this.visible || this.loading || !mediaPath) return;
    if (this.playingLine?.index === line.index && this.playerElement) {
      if (!this.playerElement.paused) { this.stopPlayback(); return; }
    }
    const request = this.playbackRequests.begin();
    this.playerElement?.pause();
    this.playingLine = line;
    this.isPlaying = false;
    try {
      const [port, token] = await this.getMediaServerInfo();
      if (!request.isCurrent() || !this.visible) return;
      let preparedPath = mediaPath;
      if (TRANSCODE_EXTENSIONS.test(mediaPath)) {
        snackbar.show(t("sync.transcodingMedia"), "info");
        preparedPath = await invoke<string>("sync_prepare_media_for_playback", { path: mediaPath });
      }
      if (!request.isCurrent() || !this.visible || !this.playerElement) return;
      const player = this.playerElement;
      player.src = `http://127.0.0.1:${port}/media?path=${encodeURIComponent(preparedPath)}&token=${token}#t=${line.start_ms / 1000},${line.end_ms / 1000}`;
      player.load();
      await player.play();
      if (request.isCurrent() && this.visible) this.isPlaying = true;
    } catch (error) {
      if (!request.isCurrent()) return;
      this.playingLine = null;
      this.isPlaying = false;
      snackbar.show(t("flashcards.previewAudioError"), "error");
      console.error("Preview playback:", error);
    }
  }

  stopPlayback() {
    this.playbackRequests.invalidate();
    this.playerElement?.pause();
    this.isPlaying = false;
    this.playingLine = null;
  }

  /** Rewrites `config`'s subtitle paths to temp files reflecting in-dialog
   * edits, but only where the config still targets what was previewed —
   * shared by both the single-file and per-episode generation paths. */
  async applyOverrides(config: { target_subs_path: string; native_subs_path?: string | null }) {
    if (this.loading || this.lines.length === 0) return;
    const lines = $state.snapshot(this.lines);
    if (config.target_subs_path === this.previewedSubsPath) {
      config.target_subs_path = await invoke<string>("save_temp_subtitles", {
        lines,
        useNative: false,
      });
    }
    if (config.native_subs_path && config.native_subs_path === this.previewedNativeSubsPath) {
      config.native_subs_path = await invoke<string>("save_temp_subtitles", {
        lines,
        useNative: true,
      });
    }
  }
}

export const previewStore = new PreviewStore();

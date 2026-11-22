import { t } from "$lib/i18n";
import { LatestRequest } from "$lib/utils/asyncControl.svelte";
import { normalizeLanguageCode } from "$lib/config/languages";
import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { snackbar } from "$lib/stores/snackbarStore.svelte";

export interface FontStatusInfo {
  id: string;
  name: string;
  language_name: string;
  target_languages: string[];
  filename: string;
  approx_size: string;
  downloaded: boolean;
}

interface FontDownloadProgressEvent {
  font_id: string;
  percentage: number;
}

class FontStore {
  private listRequests = new LatestRequest();
  deletingFontId = $state<string | null>(null);
  fonts = $state<FontStatusInfo[]>([]);
  downloadingFontId = $state<string | null>(null);
  downloadProgress = $state<number>(0);
  downloadingAll = $state(false);
  isDownloading = $derived(this.downloadingFontId !== null || this.downloadingAll || this.deletingFontId !== null);

  private unlistenProgress: UnlistenFn | null = null;

  constructor() {
    this.init();
  }

  async init() {
    await this.loadFonts();
    this.setupListener();
  }

  async setupListener() {
    if (this.unlistenProgress) return;
    try {
      this.unlistenProgress = await listen<FontDownloadProgressEvent>(
        "font-download-progress",
        (event) => {
          if (this.downloadingFontId === event.payload.font_id) {
            this.downloadProgress = event.payload.percentage;
          }
        }
      );
    } catch {
      // Ignore if running in test / non-Tauri environment
    }
  }

  async loadFonts(): Promise<boolean> {
    const request = this.listRequests.begin();
    try {
      const fonts = await invoke<FontStatusInfo[]>("flashcard_list_fonts");
      if (request.isCurrent()) this.fonts = fonts;
      return request.isCurrent();
    } catch (err) {
      console.warn("Failed to load fonts list:", err);
      return false;
    }
  }

  getFontForLanguage(lang: string): FontStatusInfo | undefined {
    const code = normalizeLanguageCode(lang);
    if (!code) return undefined;
    return this.fonts.find((font) => font.target_languages.includes(code));
  }

  isFontDownloadedForLanguage(lang: string): boolean {
    const font = this.getFontForLanguage(lang);
    return font ? font.downloaded : false;
  }

  needsSpecialFont(lang: string): boolean {
    return this.getFontForLanguage(lang) !== undefined;
  }

  async downloadFont(fontId: string): Promise<boolean> {
    if (this.isDownloading) return false;
    return this.performDownload(fontId);
  }

  async downloadAllFonts(): Promise<void> {
    if (this.isDownloading) return;
    this.downloadingAll = true;
    try {
      for (const font of this.fonts.filter((font) => !font.downloaded)) {
        await this.performDownload(font.id);
      }
    } finally {
      this.downloadingAll = false;
    }
  }

  private async performDownload(fontId: string): Promise<boolean> {
    this.downloadingFontId = fontId;
    this.downloadProgress = 0;

    try {
      if (!await invoke<boolean>("flashcard_download_font", { fontId })) throw new Error(t("setup.resourceCheckFailed"));
      if (!await this.loadFonts()) throw new Error(t("setup.resourceCheckFailed"));
      snackbar.show(t("common.downloaded", { name: this.fonts.find(font => font.id === fontId)?.name ?? fontId }), "success");
      return true;
    } catch (err) {
      console.error("Font download failed:", err);
      snackbar.show(t("common.downloadFailed", { name: fontId, error: String(err) }), "error");
      return false;
    } finally {
      this.downloadingFontId = null;
      this.downloadProgress = 0;
    }
  }

  async deleteFont(fontId: string): Promise<boolean> {
    if (this.isDownloading) return false;
    this.deletingFontId = fontId;
    try {
      await invoke<boolean>("flashcard_delete_font", { fontId });
      if (!await this.loadFonts()) throw new Error(t("setup.resourceCheckFailed"));
      snackbar.show(t("common.removed", { name: fontId }), "success");
      return true;
    } catch (err) {
      console.error("Font delete failed:", err);
      snackbar.show(t("common.removeFailed", { name: fontId, error: String(err) }), "error");
      return false;
    } finally { this.deletingFontId = null; }
  }
}

export const fontStore = new FontStore();

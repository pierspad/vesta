import type { buildFlashcardConfig } from "$lib/utils/flashcardConfig";
import type { PreviewLine } from "$lib/stores/previewStore.svelte";

type FlashcardConfig = ReturnType<typeof buildFlashcardConfig>;

export interface FlashcardPreviewOptions {
  isBusy: () => boolean;
  canPreview: () => boolean;
  isLoading: () => boolean;
  buildConfig: () => FlashcardConfig;
  load: (config: FlashcardConfig, onLoaded: (lines: PreviewLine[]) => void) => Promise<void>;
  close: () => void;
  setError: (message: string | null) => void;
  missingRequirementsMessage: () => string;
  addLog: (message: string, level: string) => void;
}

/** Owns the tab-facing preview request lifecycle; PreviewStore owns dialog data and fetching. */
export class FlashcardPreviewController {
  private disposed = false;

  constructor(private readonly options: FlashcardPreviewOptions) {}

  async load(): Promise<void> {
    const options = this.options;
    if (this.disposed || options.isBusy() || options.isLoading()) return;
    if (!options.canPreview()) {
      options.setError(options.missingRequirementsMessage());
      return;
    }

    options.setError(null);
    try {
      await options.load(options.buildConfig(), (lines) => {
        if (this.disposed) return;
        options.addLog(
          `Preview: ${lines.length} total, ${lines.filter((line) => line.active).length} active`,
          "info",
        );
      });
    } catch (error) {
      if (!this.disposed) options.setError(`Preview error: ${error}`);
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.options.close();
  }
}

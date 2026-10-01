import { loadVadSelection, type VadSelection } from "$lib/config/vadSelection";
import { transcribeAddonsStatus, transcribeCheckBackends, transcribeListModels, transcribePathExists, type TranscribeBackendsStatus, type VadModel, type WhisperModel } from "$lib/services/transcribe";

/** Owns backend/model discovery. A stale or disposed request cannot overwrite newer state. */
export class TranscriptionResources {
  backends = $state<TranscribeBackendsStatus | null>(null);
  models = $state<WhisperModel[]>([]);
  vadModels = $state<VadModel[]>([]);
  vadSelection = $state<VadSelection>(loadVadSelection());
  vadInstalled = $state(false);
  gpuSupported = $state(false);
  private modelsVersion = 0;
  private addonsVersion = 0;
  private backendsVersion = 0;
  private disposed = false;

  async refreshModels(): Promise<void> {
    const version = ++this.modelsVersion;
    try {
      const models = await transcribeListModels();
      if (!this.disposed && version === this.modelsVersion) this.models = models;
    } catch (error) { console.error("Could not list transcription models:", error); }
  }

  async refreshBackends(): Promise<void> {
    const version = ++this.backendsVersion;
    try {
      const backends = await transcribeCheckBackends();
      if (!this.disposed && version === this.backendsVersion) this.backends = backends;
    } catch (error) { console.error("Could not check transcription backends:", error); }
  }

  async refreshAddons(): Promise<void> {
    const version = ++this.addonsVersion;
    try {
      const status = await transcribeAddonsStatus();
      const selection = loadVadSelection();
      let installed = status.vad_models.some(model => model.id === selection.modelId && model.downloaded);
      if (selection.customPath) {
        try { installed = await transcribePathExists(selection.customPath); } catch { installed = false; }
      }
      if (this.disposed || version !== this.addonsVersion) return;
      this.vadModels = status.vad_models;
      this.gpuSupported = status.gpu_supported;
      this.vadSelection = selection;
      this.vadInstalled = installed;
    } catch (error) { console.error("Could not read transcription add-ons status:", error); }
  }

  dispose(): void { this.disposed = true; }
}

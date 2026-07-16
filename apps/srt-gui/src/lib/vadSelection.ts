import * as vestaConfig from "./vestaConfig";

// ─── Silero VAD variant selection ───────────────────────────────────────────
// Which VAD model to run when the user has VAD enabled: one of the built-in
// variants (see `srt_transcribe::model::VAD_MODELS`, mirrored here only by
// id) or a custom local path picked via file dialog. Shared between
// SettingsTab (download/select/upload) and TranscribeTab (resolves the
// active choice into `transcribe_start`'s config).

export const DEFAULT_VAD_MODEL_ID = "v5.1.2";

export interface VadSelection {
  /** Id of the selected built-in variant, ignored when `customPath` is set. */
  modelId: string;
  /** Absolute path to a user-provided .bin, overrides `modelId` when set. */
  customPath: string | null;
}

const VAD_SELECTION_KEY = "vesta-transcribe-vad-selection";

export function loadVadSelection(): VadSelection {
  const fallback: VadSelection = { modelId: DEFAULT_VAD_MODEL_ID, customPath: null };
  try {
    const raw = vestaConfig.getItem(VAD_SELECTION_KEY);
    if (raw) {
      const p = JSON.parse(raw);
      return {
        modelId: typeof p.modelId === "string" ? p.modelId : DEFAULT_VAD_MODEL_ID,
        customPath: typeof p.customPath === "string" ? p.customPath : null,
      };
    }
  } catch {
    /* ignore */
  }
  return fallback;
}

export function saveVadSelection(s: VadSelection): void {
  vestaConfig.setItem(VAD_SELECTION_KEY, JSON.stringify(s));
}

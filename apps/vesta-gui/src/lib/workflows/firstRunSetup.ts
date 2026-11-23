import type { WhisperModel, VadModel } from "$lib/services/transcribe";
import type { OcrModel } from "$lib/services/ocr";
import type { FontStatusInfo } from "$lib/stores/fontStore.svelte";
import { normalizeLanguageCode } from "$lib/config/languages";

export type SetupDestination = "flashcards" | "transcribe" | "extract";
export interface SetupChoices {
  mode: "quick" | "custom";
  wantsTranscription: boolean;
  whisperModel: string;
  useVad: boolean;
  vadChoice: "silero" | "custom";
  wantsOcr?: boolean;
  ocrModel?: string;
  nativeLanguage: string;
  studyLanguage: string;
}
export interface SetupResources {
  models: WhisperModel[];
  vadModels: VadModel[];
  ocrModels?: OcrModel[];
  fonts: FontStatusInfo[];
  mediaReady: boolean;
  mediaDownloadBytes: number | null;
}
export interface SetupTask {
  id: string;
  kind: "media" | "whisper" | "vad" | "ocr" | "font";
  resourceId: string;
  name: string;
  size: string;
  installed: boolean;
}
export function resolveSetupChoices(choices: SetupChoices): SetupChoices {
  const defaultOcr = choices.ocrModel || suggestedOcrModelForLanguage(choices.studyLanguage);
  return choices.mode === "quick"
    ? {
        ...choices,
        wantsTranscription: true,
        whisperModel: "small",
        useVad: true,
        vadChoice: "silero",
        wantsOcr: true,
        ocrModel: suggestedOcrModelForLanguage(choices.studyLanguage),
      }
    : {
        ...choices,
        ocrModel: defaultOcr,
      };
}
export function buildSetupPlan(raw: SetupChoices, resources: SetupResources): SetupTask[] {
  const choices = resolveSetupChoices(raw);
  const plan: SetupTask[] = [{ id: "media", kind: "media", resourceId: "ffmpeg", name: "FFmpeg + FFprobe",
    size: resources.mediaDownloadBytes === null ? "" : `~${Math.ceil(resources.mediaDownloadBytes / 1_000_000)} MB`, installed: resources.mediaReady }];
  if (choices.wantsTranscription) {
    const model = resources.models.find(m => m.id === choices.whisperModel);
    if (!model) throw new Error(`Whisper model unavailable: ${choices.whisperModel}`);
    plan.push({ id: "whisper", kind: "whisper", resourceId: model.id, name: `Whisper ${model.name}`, size: model.size, installed: model.downloaded });
    if (choices.useVad && choices.vadChoice === "silero") {
      const vad = resources.vadModels.find(m => m.id === "v6.2.0");
      if (!vad) throw new Error("Silero VAD unavailable");
      plan.push({ id: "vad", kind: "vad", resourceId: vad.id, name: "Silero VAD 6.2.0", size: vad.size, installed: vad.downloaded });
    }
  }
  if (choices.wantsOcr) {
    const ocrModelId = choices.ocrModel || suggestedOcrModelForLanguage(choices.studyLanguage);
    const ocrList = resources.ocrModels ?? [];
    const ocrModel = ocrList.find(m => m.id === ocrModelId) ?? ocrList[0];
    if (ocrModel) {
      plan.push({ id: "ocr", kind: "ocr", resourceId: ocrModel.id, name: ocrModel.name, size: ocrModel.size, installed: ocrModel.downloaded });
    }
  }
  for (const font of resources.fonts.filter(font => [choices.nativeLanguage, choices.studyLanguage].some(language => font.target_languages.includes(normalizeLanguageCode(language))))) {
    plan.push({ id: `font:${font.id}`, kind: "font", resourceId: font.id, name: font.name, size: font.approx_size, installed: font.downloaded });
  }
  return plan;
}
export type SetupTaskStatus = "pending" | "installing" | "ready" | "failed";
/** Sequential downloads. Retry plans skip verified resources from the previous attempt. */
export async function installSetupPlan(plan: SetupTask[], install: (task: SetupTask) => Promise<void>, onStatus: (task: SetupTask, status: SetupTaskStatus) => void): Promise<void> {
  for (const task of plan) {
    if (task.installed) { onStatus(task, "ready"); continue; }
    onStatus(task, "installing");
    try { await install(task); onStatus(task, "ready"); }
    catch (error) { onStatus(task, "failed"); throw error; }
  }
}
export function setupTierPreferences(model: string, wantsTranscription: boolean): string {
  return JSON.stringify(wantsTranscription ? [{ id: "setup-local", entries: [{ id: "setup-whisper", provider: "local", apiKeyId: "", model }] }] : []);
}
export function setupIttTierPreferences(ocrModel: string, wantsOcr: boolean): string {
  return JSON.stringify(
    wantsOcr
      ? [{ id: "setup-itt-local", entries: [{ id: "setup-ocr", provider: "local_ocr", apiKeyId: "local_ocr", model: ocrModel }] }]
      : [],
  );
}

const CJK_OCR_LANGUAGES = new Set(["zh", "zh-tw", "ja", "ko"]);
const LATIN_OCR_LANGUAGES = new Set([
  "ca", "cs", "da", "de", "en", "es", "fi", "fr", "hu", "id", "is", "it",
  "ms", "nl", "no", "pl", "pt", "pt-br", "ro", "sv", "tr", "vi",
]);

/** Recommends the best local PaddleOCR model for a given study language code. */
export function suggestedOcrModelForLanguage(languageCode: string): string {
  const normalized = normalizeLanguageCode(languageCode) || languageCode.trim().toLowerCase();
  if (CJK_OCR_LANGUAGES.has(normalized)) return "paddleocr-v4-mobile-cjk";
  if (LATIN_OCR_LANGUAGES.has(normalized)) return "paddleocr-v4-mobile-latin";
  return "paddleocr-v4-mobile-multi";
}

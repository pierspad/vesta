import type { WhisperModel, VadModel } from "$lib/services/transcribe";
import type { FontStatusInfo } from "$lib/stores/fontStore.svelte";
import { normalizeLanguageCode } from "$lib/config/languages";

export type SetupDestination = "flashcards" | "transcribe" | "extract";
export interface SetupChoices {
  mode: "quick" | "custom";
  wantsTranscription: boolean;
  whisperModel: string;
  useVad: boolean;
  vadChoice: "silero" | "custom";
  nativeLanguage: string;
  studyLanguage: string;
}
export interface SetupResources {
  models: WhisperModel[];
  vadModels: VadModel[];
  fonts: FontStatusInfo[];
  mediaReady: boolean;
  mediaDownloadBytes: number | null;
}
export interface SetupTask {
  id: string;
  kind: "media" | "whisper" | "vad" | "font";
  resourceId: string;
  name: string;
  size: string;
  installed: boolean;
}
export function resolveSetupChoices(choices: SetupChoices): SetupChoices {
  return choices.mode === "quick"
    ? { ...choices, wantsTranscription: true, whisperModel: "small", useVad: true, vadChoice: "silero" }
    : { ...choices };
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

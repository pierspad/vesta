import { invokeCommand } from "$lib/services/tauriClient";
import type { TierEntryPayload } from "$lib/config/llmTiers";

/** Comandi Rust del dominio "translate" (traduzione LLM di sottotitoli .srt). */

export interface SrtFileInfo {
  path: string;
  subtitle_count: number;
  first_subtitle: string;
  last_subtitle: string;
}

export interface TranslateConfig {
  input_path: string;
  output_path: string;
  target_lang: string;
  batch_size: number;
  resume_overlap: number;
  title_context: string | null;
  tiers: TierEntryPayload[][];
}

export interface TranslateResult {
  success: boolean;
  message: string;
  output_path: string | null;
  translated_count: number;
}

export interface SubtitlePair {
  id: number;
  original: string;
  translated: string;
}

export function getLatestTranslatedSubtitles(
  inputPath: string,
  outputPath: string,
  count: number,
): Promise<SubtitlePair[]> {
  return invokeCommand<SubtitlePair[]>("get_latest_translated_subtitles", {
    inputPath,
    outputPath,
    count,
  });
}

export function loadSrtForTranslate(path: string): Promise<SrtFileInfo> {
  return invokeCommand<SrtFileInfo>("load_srt_for_translate", { path });
}

export function startTranslation(config: TranslateConfig): Promise<TranslateResult> {
  return invokeCommand<TranslateResult>("start_translation", { config });
}

export function cancelTranslation(): Promise<void> {
  return invokeCommand<void>("cancel_translation");
}

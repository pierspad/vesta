import { invokeCommand } from "$lib/services/tauriClient";

/**
 * Comandi Rust del dominio "transcribe": modelli Whisper, add-on VAD, verifica
 * dei backend disponibili e avvio/annullamento della trascrizione vera e
 * propria.
 *
 * Perché un file solo: `transcribe_check_file_exists`, `transcribe_cancel` e
 * `transcribe_check_backends` erano invocati in modo identico sia da
 * `TranscribeTab.svelte` sia da `TranslateTab.svelte`/`whisperModelsStore` --
 * stessa stringa di comando, stesso payload, copiati a mano. Centralizzarli
 * qui elimina la duplicazione e rende visibile a colpo d'occhio l'intera
 * superficie di comandi "transcribe_*" esposta da Rust.
 */

export interface WhisperModel {
  id: string;
  name: string;
  size: string;
  speed: string;
  downloaded: boolean;
}

export interface VadModel {
  id: string;
  size: string;
  downloaded: boolean;
}

export interface TranscribeBackendsStatus {
  ffmpeg: boolean;
  whisper_cpp: boolean;
  python_whisper: boolean;
  any_whisper: boolean;
  whisper_binary: string | null;
}

export interface TranscribeAddonsStatus {
  vad_models: VadModel[];
  gpu_supported: boolean;
}

export interface TranscribeConfig {
  input_path: string;
  output_path: string;
  model: string;
  language: string;
  translate_to_english: boolean;
  word_timestamps: boolean;
  max_segment_length: number;
  provider: string;
  api_key: string | null;
  api_url: string | null;
  quality: boolean;
  vad: boolean;
  vad_model_id: string | null;
  vad_custom_path: string | null;
  use_gpu: boolean;
}

export interface TranscribeStartResult {
  success: boolean;
  message: string;
  output_path?: string;
  subtitle_count?: number;
  detected_language?: string;
}

export function transcribeListModels(): Promise<WhisperModel[]> {
  return invokeCommand<WhisperModel[]>("transcribe_list_models");
}

export function transcribeCheckBackends(): Promise<TranscribeBackendsStatus> {
  return invokeCommand<TranscribeBackendsStatus>("transcribe_check_backends");
}

export function transcribeAddonsStatus(): Promise<TranscribeAddonsStatus> {
  return invokeCommand<TranscribeAddonsStatus>("transcribe_addons_status");
}

export function transcribePathExists(path: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_path_exists", { path });
}

export function transcribeCheckFileExists(path: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_check_file_exists", { path });
}

export function transcribeDownloadModel(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_download_model", { modelId });
}

export function transcribeUninstallModel(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_uninstall_model", { modelId });
}

export function transcribeDownloadVad(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_download_vad", { modelId });
}

export function transcribeUninstallVad(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("transcribe_uninstall_vad", { modelId });
}

export function transcribeStart(config: TranscribeConfig): Promise<TranscribeStartResult> {
  return invokeCommand<TranscribeStartResult>("transcribe_start", { config });
}

export function transcribeCancel(): Promise<void> {
  return invokeCommand<void>("transcribe_cancel");
}

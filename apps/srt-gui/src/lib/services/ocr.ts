import { invokeCommand } from "$lib/services/tauriClient";
import type { TierEntryPayload } from "$lib/config/llmTiers";

export interface OcrModel {
  id: string;
  name: string;
  size: string;
  downloaded: boolean;
}

export interface VideoRoi {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface VideoFramePreview {
  data_url: string;
  width: number;
  height: number;
  duration_ms: number;
  time_ms: number;
}

export interface HardcodedOcrPreviewRequest {
  path: string;
  roi: VideoRoi;
  sampleFps: number | null;
  language: string | null;
  tiers: TierEntryPayload[][];
}

export interface HardcodedOcrRequest extends HardcodedOcrPreviewRequest {
  outputPath: string;
}

export function ocrListModels(): Promise<OcrModel[]> {
  return invokeCommand<OcrModel[]>("ocr_list_models");
}

export function ocrDownloadModel(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("ocr_download_model", { modelId });
}

export function ocrUninstallModel(modelId: string): Promise<boolean> {
  return invokeCommand<boolean>("ocr_uninstall_model", { modelId });
}

export function ocrCancel(): Promise<boolean> {
  return invokeCommand<boolean>("ocr_cancel");
}

export function ocrVideoPreviewFrame(path: string, timeMs: number | null = null): Promise<VideoFramePreview> {
  return invokeCommand<VideoFramePreview>("ocr_video_preview_frame", { path, timeMs });
}

export function previewOcrHardcodedSubtitle(params: HardcodedOcrPreviewRequest): Promise<string> {
  return invokeCommand<string>("preview_ocr_hardcoded_subtitle", { ...params });
}

export function ocrHardcodedSubtitle(params: HardcodedOcrRequest): Promise<void> {
  return invokeCommand<void>("ocr_hardcoded_subtitle", { ...params });
}

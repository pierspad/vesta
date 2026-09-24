export interface SystemDiagnostics {
  os: string;
  arch: string;
  compute_backend: string;
  gpu_compiled: boolean;
  gpu_device: string | null;
  ffmpeg_available: boolean;
  video_encoder: string;
  hardware_video_encoder: boolean;
  gstreamer_available: boolean;
  gstreamer_h264: boolean;
  gstreamer_h265: boolean;
}

export function displayComputeBackend(diagnostics: SystemDiagnostics): string {
  if (!diagnostics.gpu_compiled || diagnostics.compute_backend === "none") return "CPU";
  const backend = diagnostics.compute_backend.toUpperCase();
  return diagnostics.gpu_device ? `${backend} · ${diagnostics.gpu_device}` : backend;
}

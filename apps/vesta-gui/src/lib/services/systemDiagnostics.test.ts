import { describe, expect, it } from "vitest";
import { displayComputeBackend, type SystemDiagnostics } from "./systemDiagnostics";

const base: SystemDiagnostics = {
  os: "linux", arch: "x86_64", compute_backend: "cpu", gpu_compiled: false,
  gpu_device: null, ffmpeg_available: true, video_encoder: "libx264 (CPU)",
  hardware_video_encoder: false, gstreamer_available: true,
  gstreamer_h264: true, gstreamer_h265: true,
};

describe("displayComputeBackend", () => {
  it("reports the CPU fallback", () => expect(displayComputeBackend(base)).toBe("CPU"));
  it("includes the runtime GPU name when available", () => {
    expect(displayComputeBackend({ ...base, gpu_compiled: true, compute_backend: "vulkan", gpu_device: "AMD Radeon" }))
      .toBe("VULKAN · AMD Radeon");
  });
});

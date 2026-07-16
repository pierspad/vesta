pub mod audio;
pub mod cloud;
pub mod model;
pub mod pipeline;
pub mod transcribe;

/// True when this build includes a GPU backend (Vulkan) for whisper.cpp.
///
/// Compile with `--features vulkan` to enable it; at runtime the GPU is only
/// used when the caller also opts in (see `TranscriptionConfig::use_gpu`),
/// and whisper.cpp falls back to CPU when no usable device is found.
pub const fn gpu_supported() -> bool {
    cfg!(feature = "vulkan")
}

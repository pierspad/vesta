pub mod audio;
pub mod cloud;
pub mod model;
pub mod pipeline;
pub mod transcribe;

/// Whether this build carries any GPU offload backend.
pub const fn gpu_supported() -> bool {
    cfg!(any(feature = "vulkan", feature = "cuda", feature = "rocm"))
}

/// Name of the GPU backend compiled into this build ("none" for CPU-only).
/// Mutually exclusive in practice: CI builds one backend per binary.
pub const fn gpu_backend_name() -> &'static str {
    if cfg!(feature = "cuda") {
        "cuda"
    } else if cfg!(feature = "rocm") {
        "rocm"
    } else if cfg!(feature = "vulkan") {
        "vulkan"
    } else {
        "none"
    }
}

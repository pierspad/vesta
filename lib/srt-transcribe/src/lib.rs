pub mod audio;
pub mod cloud;
pub mod model;
pub mod pipeline;
pub mod transcribe;

/// Whether this build carries any GPU offload backend.
pub const fn gpu_supported() -> bool {
    cfg!(any(
        feature = "vulkan",
        feature = "cuda",
        feature = "rocm",
        feature = "sycl"
    ))
}

/// Name of the GPU backend compiled into this build ("none" for CPU-only).
/// Mutually exclusive in practice: each binary is built with exactly one of
/// these features (see apps/whisper-bench, which bundles the `vulkan` build
/// as its launcher and fetches single-backend `cuda`/`rocm`/`sycl` worker
/// binaries on demand rather than linking them all in at once).
pub const fn gpu_backend_name() -> &'static str {
    if cfg!(feature = "cuda") {
        "cuda"
    } else if cfg!(feature = "rocm") {
        "rocm"
    } else if cfg!(feature = "sycl") {
        "sycl"
    } else if cfg!(feature = "vulkan") {
        "vulkan"
    } else {
        "none"
    }
}

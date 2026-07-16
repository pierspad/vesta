pub mod audio;
pub mod cloud;
pub mod model;
pub mod pipeline;
pub mod transcribe;

pub const fn gpu_supported() -> bool {
    cfg!(feature = "vulkan")
}

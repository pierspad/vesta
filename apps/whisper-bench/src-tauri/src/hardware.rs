//! Best-effort runtime hardware detection.
//!
//! This only decides *which* variants are worth attempting (and, for
//! CUDA/ROCm/SYCL, whether it's worth downloading the matching worker
//! binary at all) — it is deliberately soft. A false positive just means a
//! variant gets attempted and recorded as `error` (the benchmark loop
//! already tolerates that for every variant); a false negative just means a
//! usable backend is skipped. Neither case corrupts results, so simple
//! heuristics (parsing `lspci`/CIM adapter names) are good enough and avoid
//! pulling in vendor SDKs just to ask "is an AMD GPU present".

use std::process::Stdio;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct DetectedHardware {
    pub nvidia: bool,
    pub amd: bool,
    pub intel_gpu: bool,
    pub vulkan_loader: bool,
}

pub async fn detect() -> DetectedHardware {
    let names = gpu_adapter_names().await;
    let has = |needle: &str| names.iter().any(|n| n.to_lowercase().contains(needle));

    DetectedHardware {
        nvidia: has("nvidia") || command_ok("nvidia-smi", &["-L"]).await,
        amd: has("amd") || has("radeon") || has("advanced micro devices"),
        intel_gpu: has("intel"),
        vulkan_loader: vulkan_loader_present().await,
    }
}

async fn command_ok(cmd: &str, args: &[&str]) -> bool {
    tokio::process::Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Best-effort list of GPU adapter name strings, used only to sniff vendors.
async fn gpu_adapter_names() -> Vec<String> {
    if cfg!(target_os = "windows") {
        let out = tokio::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
            ])
            .output()
            .await;
        out.ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        let out = tokio::process::Command::new("lspci")
            .arg("-nn")
            .output()
            .await;
        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| l.contains("VGA compatible controller") || l.contains("3D controller"))
                .map(str::to_string)
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Whether a working Vulkan loader/ICD is likely present. Compiled-in Vulkan
/// support (`gpu_supported()`) only means the launcher *can* use Vulkan, not
/// that this particular machine has a usable ICD — running the variant
/// anyway on a headless/no-GPU box just produces a confusing `error` row, so
/// we check first.
async fn vulkan_loader_present() -> bool {
    if command_ok("vulkaninfo", &["--summary"]).await {
        return true;
    }
    // `vulkaninfo` ships in a separate `vulkan-tools`-style package and may
    // be absent even with a perfectly good loader installed; fall back to
    // checking for the loader library itself.
    if cfg!(target_os = "windows") {
        std::path::Path::new(r"C:\Windows\System32\vulkan-1.dll").exists()
    } else {
        [
            "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
            "/usr/lib/libvulkan.so.1",
            "/usr/lib64/libvulkan.so.1",
            "/usr/lib/aarch64-linux-gnu/libvulkan.so.1",
        ]
        .iter()
        .any(|p| std::path::Path::new(p).exists())
    }
}

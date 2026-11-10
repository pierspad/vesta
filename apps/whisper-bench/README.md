# whisper-bench

Standalone Tauri mini-application for benchmarking Vesta's Whisper transcription engine.

**A single binary per OS.** Select an audio or video file (or click
**Use bundled sample episode** to use a pre-packaged sample), click
**Run benchmark**, and the application automatically detects available hardware
and executes all relevant transcription variants for your machine.

| Always bundled in launcher | Downloaded on demand when detected |
|---|---|
| `cpu`, `cpu+vad` | — |
| `vulkan`, `vulkan+vad` (when compiled with `--features vulkan` and drivers/loaders are operational) | — |
| — | `cuda`, `cuda+vad` (NVIDIA GPU detected) |
| — | `rocm`, `rocm+vad` (AMD GPU detected, Linux only) |
| — | `sycl`, `sycl+vad` (Intel GPU detected) |

The launcher binary bundles CPU + Vulkan support (a single binary covering AMD, NVIDIA, and Intel without requiring dedicated CUDA/ROCm SDK installations). CUDA, ROCm, and SYCL runtimes are never bundled into the launcher binary; instead, they are built as standalone `cli/srt-transcribe-cli` worker binaries using the exact same transcription pipeline (`lib/srt-transcribe`) with `--features cuda|rocm|sycl`. These workers are built separately, cached locally after SHA-256 verification, and downloaded from the corresponding release only when matching hardware is detected. Hardware detection is best-effort (`nvidia-smi`, `lspci`/WMI, and presence of the Vulkan loader); a false positive merely logs an `error` entry in the result table without affecting anything else.

Each variant reports wall-clock execution time, realtime factor, subtitle line count, and detected language; clicking **Download CSV** exports the benchmark table. The Whisper model, Silero VAD model, and a static build of FFmpeg (if not already found on PATH) are downloaded automatically on first launch.

The top status bar continuously displays the current activity (hardware detection, asset/worker downloads, active variant and completion percentage) so that the UI never appears frozen or empty. The detail log below provides line-by-line output; if a variant encounters an error, it is recorded as `error` with the full diagnostic message, and benchmarking continues with subsequent variants (useful for identifying non-functional GPU configurations).

## Local Builds

```bash
# Launcher (standard application binary)
cd apps/whisper-bench/src-tauri
npx @tauri-apps/cli@^2 build -- --features vulkan

# Headless worker for heavy backend testing locally
cargo build --release --features cuda -p srt-transcribe-cli
```

## CI Builds

The `whisper-bench` GitHub Actions workflow (Actions → whisper-bench → Run workflow):

- `launcher`: Builds Linux and Windows binaries with CPU + Vulkan enabled.
- `worker`: Builds `srt-transcribe-cli` with `--features cuda|rocm|sycl` as opt-in choices from manual workflow dispatch checkboxes (due to large toolchains and slow compile times), running with `continue-on-error`. These backends are only compiled in CI (GitHub runners lack physical GPUs) and not tested against real hardware.
- `zip-sample`: Packages the test clip in `Test_Subs/fixtures/detour` into an in-app downloadable sample.

All assets are uploaded to the rolling `whisper-bench-latest` prerelease tag, which the launcher references at runtime to resolve worker and sample URLs.

The default Cargo profile uses `default = []` (CPU only). The presence of a Vulkan loader does not enable Vulkan acceleration in a CPU-only build; launcher releases require the explicit `vulkan` feature. Workspace test suites do not substitute for actual GPU inference validation on real hardware.

# Vesta CLI

One headless entry point for Vesta's seven existing command-line workflows.
It calls their Rust implementations directly and does not depend on Tauri.

```sh
cargo install --path cli/vesta-cli
vesta-cli --help
vesta-cli flashcards generate --help
vesta-cli translate --help
```

Operations: `flashcards`, `extract`, `sync`, `autosync`, `transcribe`,
`translate`, `refine`. Each exposes its full options with `--help`.
The individual `srt-*` executables remain supported.

The desktop executable also accepts these operations as `vesta <operation>`;
running `vesta` without arguments opens the GUI. On Windows, use the standalone
`vesta-cli.exe` to receive terminal output.

Media workflows need FFmpeg/FFprobe. Transcription and automatic alignment need
the appropriate model or configured cloud provider. Local transcription uses
the CPU by default; optional `vulkan`, `cuda`, `rocm` and `sycl` Cargo features
enable the corresponding native build requirements.

For embedding the engines rather than invoking commands, see
[module APIs](../../docs/modules/README.md) and
[integration requirements](../../docs/library-integration.md).

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.

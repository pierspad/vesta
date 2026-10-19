# Using Vesta Rust libraries downstream

Vesta's parser, extraction, translation, synchronization, transcription, autosync, flashcard, AnkiConnect, and APKG crates can be consumed independently of the desktop application. The integration smoke project under `build-scripts/library-smoke` is deliberately outside Cargo's workspace when the check runs: it creates a temporary Cargo project and points its dependencies at the library directories by absolute path.

Run the downstream compile and API smoke check from any working directory:

```sh
python3 /path/to/vesta/build-scripts/check_library_integration.py
```

Use `--offline` when all required crates are already cached:

```sh
python3 /path/to/vesta/build-scripts/check_library_integration.py --offline
```

The check resolves all 11 core and library crates, rejects Tauri/GTK/WebKit in the resolved dependency graph, confirms that no Whisper GPU backend is enabled, and compiles/runs a small consumer. Its operations are local: it does not contact providers, download models, access Anki, or invoke FFmpeg.

Add `--individual` to also create a separate temporary consumer and run `cargo check` for each crate before the combined smoke project. This catches dependency or feature assumptions hidden by feature unification in the combined graph.

## Git dependencies

For an application that consumes the whole library set, pin a Git dependency to a tag or immutable revision rather than a moving branch. For example:

```toml
[dependencies]
srt-parser = { git = "https://github.com/pierspad/vesta", rev = "<commit-sha>" }
```

This pattern is convenient for a small consumer of one crate. For several crates from one repository, a Cargo workspace dependency table or a local checkout with path dependencies can avoid repeating the repository and revision. Check each crate's manifest for its dependency and feature requirements.

Copying a crate directory by itself is insufficient: manifests inherit edition, Rust version, and dependencies through `*.workspace = true`, and internal Vesta crates are declared as workspace dependencies. Cargo resolves those declarations from the repository workspace for Git dependencies. A copied folder has neither that workspace context nor the sibling source crates.

The repository root contains a `[patch.crates-io]` override for `whisper-rs-sys`, used by `srt-transcribe` and `srt-autosync`. Cargo patches are local to the root manifest that declares them; they do not propagate to a downstream consumer. The smoke fixture repeats the patch with an explicit local path. To reproduce Vesta's supported Whisper sys build from a Git consumer, apply the same repository revision as a patch:

```toml
[patch.crates-io]
whisper-rs-sys = { git = "https://github.com/pierspad/vesta", rev = "<same-commit-sha>" }
```

Cargo discovers the nested `whisper-rs-sys` package in that repository. Keep the patch revision aligned with the Vesta crate revision. This patch is required to reproduce Vesta's supported build configuration for those crates; it is not a transitive promise made by their manifests.

Transcription needs a supported native toolchain and system libraries for the selected Whisper backend. The default configuration is CPU-only. GPU features (`vulkan`, `cuda`, `rocm`, `sycl`) are opt-in and carry backend-specific SDK/compiler requirements. Flashcard media generation and autosync also invoke FFmpeg/FFprobe at runtime; the smoke check covers their non-media public APIs without invoking those tools. Library license metadata remains in each crate manifest; retain and review it as part of your application's license and distribution process.

## Smoke coverage

| Crate | Local downstream check |
| --- | --- |
| `srt-parser` | Parse, save, and reload SRT data |
| `srt-apkg` | ZIP and unpack a package fixture |
| `srt-download` | Reuse a cached file without a network request |
| `srt-ankiconnect` | Construct a public note value; no Anki server required |
| `srt-autosync` | Construct public outcome type; model/audio work requires external assets |
| `srt-extract` | Calculate stats and export JSON |
| `srt-flashcards` | Run deterministic subtitle preview |
| `srt-refine` | Interpolate a prompt and strip HTML |
| `srt-sync` | Add an anchor, map a subtitle, and sample the next index |
| `srt-transcribe` | Deserialize config and confirm the default build is CPU-only |
| `srt-translate` | Build a local provider pool and read provider defaults |

The smoke check validates source integration and public APIs. It does not establish publication readiness, contact external services, download models, access Anki, or invoke FFmpeg. License metadata remains in each crate manifest; retain and review it as part of your application's license and distribution process.

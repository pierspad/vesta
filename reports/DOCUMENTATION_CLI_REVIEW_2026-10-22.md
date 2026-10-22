# Documentation and command-line review — 22 October 2026

Baseline: `5da45a4`. Reviewed README, documentation navigation, all 11 module
pages, the seven CLI workflows and their leaf commands, and the MCP adapter.

## Corrected findings

- README's documentation map omitted CLI coverage, MCP, downstream integration
  and the provider review. The provider review was incorrectly described in the
  documentation index as a public Rust API migration policy.
- Added `docs/CLI.md`: command-by-command usage, requirements, output/exit
  contracts, concrete misuse corrections, and a GUI/CLI/library coverage matrix.
  Clarified independent CLI configuration and MCP's narrower scope.
- Removed translation README references to nonexistent `build_all.sh` and
  unverified standalone release binaries; corrected root-relative installation.
- Corrected module API/behavior claims: nonexistent extraction format and stats,
  generic flat ZIP packing versus complete Anki package creation, explicit
  AnkiConnect deck/model setup, parser normalization bounds and charset heuristics.
- Made library examples self-contained where necessary, propagated Auto-Sync
  anchor errors/cancellation, used real subtitle IDs and absolute corrected times
  in Sync, and used the fallible translation-client constructor.
- Expanded help/examples for command leaves and both entry points. Preserved
  flashcard preview's accepted arguments; its required but unused output path
  is now explained instead of silently changing the CLI contract.
- Fixed `translate --language-list` incorrectly requiring input. Conflicting
  translation/check/list options and report-versus-embedded extraction options
  are rejected rather than silently ignored.
- Fixed missing `[output]` in translation TOML yielding an empty filename pattern.
  It now uses the same intended `{input_name}.{language}.srt` default as an empty
  output table; regression coverage checks both forms.
- Transcription reports missing custom URLs/hosted credentials and local-only
  flags before media processing. Validation failures preserve JSON mode's single
  terminal stdout record and quiet stderr. Provider aliases/case remain supported.
- MCP help now explains existing roots, stdio, tool scope and terminal alternatives.

## Coverage remains explicit

`vesta-mcp` still exists and exposes six structured tools for subtitle inspection,
paged flashcard previews, export jobs, status and cancellation. It is optional,
local, separately built, and does not expose translation/transcription or GUI
configuration. It shares the same flashcard engine rather than adding a separate
processing implementation.

CLI adapters cover the seven main workflows, but not every GUI/library feature.
In particular, interactive revision/selection, multi-episode orchestration/APKG
merge, custom card templates/field controls, transcription tiers, and AnkiConnect
are not all available as CLI commands. These are documented boundaries, not
features implemented by this documentation pass.

## Validation

- Executed all 18 top-level/operation/leaf/MCP help screens: success and usage present.
- Compiled all 10 module API examples plus the translation diagnostics example
  in a temporary local consumer with `cargo check --offline --examples`.
- Checked local Markdown links and heading anchors across README, docs and crate
  READMEs; no missing targets found.
- `cargo test --locked --workspace --exclude vesta --exclude whisper-bench`:
  330 tests passed, zero failures (including CLI subprocess and real MCP stdio tests).
- Strict Clippy for the same headless workspace with `--all-targets -- -D warnings` passed.
- Native desktop `cargo check --locked -p vesta`, Rust formatting and whitespace checks passed.

No authenticated provider calls, new model downloads, GPU inference, installed
GUI/Windows terminal checks or external advisory review were performed. Examples
were compiled, not executed against users' files or providers. Headless tests
cover offline fixtures and loopback services; they do not establish universal
codec/provider/platform compatibility.

# Prompt for a future crate naming migration

Current recommendation: retain the existing `srt-*` package names while the
new `vesta` / `vesta-cli` entry points provide a consistent product interface.
A crate rename changes downstream dependency declarations even when the Rust
API is unchanged. Plan it as a separate compatibility migration.

Copy this prompt into a repository-aware coding session:

> Review the Vesta Cargo workspace and propose a coherent crate naming scheme.
> Some `srt-*` crates now handle media, Anki packages and speech, so assess
> whether `vesta-*` names communicate their responsibilities more accurately.
> Read repository instructions, manifests, dependency graphs, public APIs,
> feature flags, CLI names, release tooling and integration documentation first.
> Produce a complete old/new name table and a compatibility plan before editing.
> Do not assume that proposed names are available on crates.io; verify availability
> if publication is in scope. Do not publish packages or delete existing names.
> Prefer a migration preserving existing CLI aliases and explaining downstream
> dependency/import changes. Avoid mixing behavioral refactoring into renames.
> If implementing, update package names, Rust imports, path/version dependencies,
> workspace declarations, acceleration features, lockfile, CI, release/version
> scripts, examples and documentation consistently. Preserve public behavior.
> Validate Cargo metadata, workspace tests, formatting, Clippy, standalone CLI
> help/generation, downstream embedding checks and internal version consistency.
> Report compatibility breaks, validation evidence and any publication steps
> that remain manual. If the migration offers little practical benefit, explain
> why retaining current crate names is the better choice.

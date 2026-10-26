# srt-download

Download mechanics independent from font and model catalogs or desktop events.

It verifies or reuses an existing nonempty cache file, streams HTTP responses into a unique temporary sibling file (`.vesta-download-...`), synchronizes and flushes writes to disk, and publishes completed files via `persist_noclobber` without clobbering concurrent results. It supports cooperative cancellation through `tokio_util::sync::CancellationToken` and automatically cleans up partial temporary files when cancelled or dropped.

The crate is GUI-independent and depends on `anyhow`, `futures`, `reqwest`, `tempfile`, `tokio`, and `tokio-util`. Workspace crates should use the root `workspace.dependencies` entry; external users can depend on the Git repository:

```toml
[dependencies]
srt-download = { git = "https://github.com/pierspad/vesta" }
```

```bash
cargo test -p srt-download
```

See [`docs/modules/srt-download.md`](../../docs/modules/srt-download.md) for module details and loopback test coverage. Licensed GPL-3.0-only.

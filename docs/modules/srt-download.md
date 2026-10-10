# srt-download

Shared font/model download mechanics with no catalog or desktop dependencies.

`download_to(url, path, progress, cancel)` reuses a regular nonempty cache file,
streams HTTP into a unique temporary sibling and publishes the completed file
without overwriting a concurrent result. HTTP, read, write, flush and publication
errors propagate. Empty downloads fail. Completion progress is emitted only after
publication; cancellation or dropping the future removes its owned temporary file.
Connection and read timeouts also cover stalled servers.

A nonempty cache is a reuse policy, not checksum validation. The caller chooses
the URL and destination, and is responsible for resource identity and authenticity.

Run `cargo test -p srt-download` for loopback tests of success/cache reuse, HTTP
failures, truncated/empty bodies, stalled headers/body cancellation, future abort
and concurrent publication. No real remote assets are needed.

For standalone reuse copy `core/srt-download/` and supply its inherited workspace
metadata/dependencies: `anyhow`, `futures`, `reqwest` with `stream`, `tempfile`,
`tokio` and `tokio-util`.

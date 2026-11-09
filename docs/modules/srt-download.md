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

## Use as a Rust dependency

```toml
[dependencies]
srt-download = { git = "https://github.com/pierspad/vesta" }
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
```

```rust
use std::path::Path;
use srt_download::download_to;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cancel = CancellationToken::new();
    download_to(
        "https://example.com/resource.bin",
        Path::new("cache/resource.bin"),
        |downloaded, total| eprintln!("{downloaded}/{total} bytes"),
        &cancel,
    )
    .await?;
    Ok(())
}
```

When vendoring `core/srt-download/` into another workspace, supply its inherited
workspace metadata and dependencies (`anyhow`, `futures`, `reqwest` with
`stream`, `tempfile`, `tokio`, `tokio-util`); see
[`docs/library-integration.md`](../library-integration.md).

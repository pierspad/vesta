# srt-parser

Foundational subtitle parser used throughout Vesta. It reads SRT files with automatic character-set detection, normalizes subtitle timing and identifiers, and writes valid UTF-8 SRT output.

The crate is GUI-independent and depends on `anyhow`, `serde`, `encoding_rs`,
`chardetng`, `tempfile`, and `same-file`. Workspace crates should use the root
`workspace.dependencies` entry; external users can depend on the Git repository:

```toml
[dependencies]
srt-parser = { git = "https://github.com/pierspad/vesta" }
```

```rust
use srt_parser::SrtParser;

fn main() -> anyhow::Result<()> {
    let mut subtitles = SrtParser::parse_file("movie.srt")?;
    SrtParser::normalize_subtitles(&mut subtitles);
    SrtParser::save_file("normalized.srt", &subtitles)?;
    Ok(())
}
```

`save_file` publishes through a temporary sibling: failed serialization, writes,
flush or replacement leave the previous destination intact. The `persistence`
module exposes `atomic_write` and `atomic_write_with` for companion exports and
session files. Buffered callbacks must finish and flush their writers before
returning. Existing symlinks are followed; dangling links and read-only targets
are rejected. Permission bits are preserved for existing targets; new files use
restrictive temporary-file permissions. Ownership, extended ACLs, concurrent
writer coordination and directory durability after power loss are outside this
contract.

`ensure_distinct_files` rejects input/output aliases (including hard links),
so conversion and resume adapters cannot mistake their source for an output.
It checks file identity before processing, without locking paths against
replacement by another process.

See [`docs/modules/srt-parser.md`](../../docs/modules/srt-parser.md) for the public data model and extraction notes. Licensed GPL-3.0-only.

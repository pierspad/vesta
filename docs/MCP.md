# Using Vesta from an MCP client

MCP (Model Context Protocol) lets an assistant discover a program's operations,
read their input schemas, and invoke them with structured parameters. The
assistant interprets the user's request; Vesta executes the actual subtitle and
media processing. See the [official architecture guide](https://modelcontextprotocol.io/docs/learn/architecture).

Vesta provides a local **server**, `vesta-mcp`, using the
[official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk). An MCP-capable
application acts as the client and starts it as a subprocess. Communication uses
JSON-RPC over stdin/stdout. No GUI or listening network port is required. The
server shares the Rust flashcard engine with the GUI and CLI, rather than
automating windows or parsing terminal output.

## Why keep MCP alongside the CLI?

Both adapters use the same engine: MCP does not improve media quality, speed or
LLM pricing. A shell-capable agent can use the CLI directly. MCP is useful for
clients that discover tools and their JSON schemas but do not expose a shell,
and for callers that want typed results, job IDs, progress and cancellation
without parsing terminal output. Keeping this adapter optional preserves both
workflows without duplicating the processing engine.

The client starts a local subprocess for its connection. That process uses local
memory while connected and exits on disconnect; no remote server subscription,
listening port or system background service is required.

## Build and connect

From the repository root:

```sh
cargo install --locked --path cli/vesta-mcp
vesta-mcp --help
```

Use the equivalent server entry in your client's MCP settings (the surrounding
configuration format depends on the client):

```json
{
  "mcpServers": {
    "vesta": {
      "command": "/absolute/path/to/vesta-mcp",
      "args": [
        "--read-root", "/absolute/path/to/media",
        "--output-root", "/absolute/path/to/exports"
      ]
    }
  }
}
```

Use `vesta-mcp.exe` on Windows. Both roots must be existing directories. Repeat
`--read-root` for additional input directories. The server exposes no application
credentials or arbitrary shell execution. Subtitle paths are canonicalized and
must remain inside a declared read root, including when symlinks are involved.
Subtitle inputs are limited to 16 MiB and supported subtitle extensions.
These checks are an application policy, not an OS sandbox against a separate
process deliberately changing files or symlinks during execution.

The server is optional and independent of `vesta` / `vesta-cli`. Running
`vesta` without arguments still opens the desktop app. A bare `vesta-mcp` requires
its directory arguments; a configured server then waits for an MCP client.

## Tools

| Tool | Purpose | Writes files |
|---|---|---|
| `get_configuration` | Read allowed roots and server limits | No |
| `inspect_subtitles` | Read format, line count, duration, actors and sample text | No |
| `preview_flashcards` | Match target/native subtitles and return a page of preview rows | No |
| `generate_flashcards` | Start a TSV/APKG export and return a job ID immediately | New directory |
| `get_job` | Read progress, status, result and export paths | No |
| `cancel_job` | Request cancellation; poll until a terminal status | May stop ongoing output |

Typical workflow:

1. Discover the tools and call `get_configuration`.
2. Inspect subtitle files and call `preview_flashcards` with a target path, an
   optional native subtitle path, `offset` (default 0), and `limit` (default 20,
   maximum 100).
3. With a user-authorized export, call `generate_flashcards`:

   ```json
   {
     "target": "/absolute/path/to/media/episode-en.srt",
     "native": "/absolute/path/to/media/episode-it.srt",
     "video": "/absolute/path/to/media/episode.mp4",
     "deck": "Episode — English",
     "format": "apkg",
     "include_audio": true,
     "include_snapshots": true,
     "workers": 4
   }
   ```

   `format` is `apkg` or `tsv`. `note_type` optionally sets the Anki note type.
   An optional `audio` input can supply separate audio. Media flags default to
   false: audio requires an audio/video input; snapshots and video clips require
   a video input. FFmpeg/FFprobe must be on PATH when processing media. Workers
   must be 1–64 and are also capped by the engine to available logical cores.
   Font downloads are disabled in this adapter.

4. Poll `get_job` with the returned `job_id`. Its `progress` is the latest engine
   event; `result` includes the card count and export paths after success.
5. Use `cancel_job` if needed. `cancelling` means the stop request was accepted;
   wait for `cancelled`, `failed` or `succeeded` before starting another job.

Statuses are `running`, `cancelling`, `succeeded`, `failed`, `cancelled`.
A completed success remains successful if cancellation arrives afterward.

## Lifetime and scope

One generation may run at a time per server process. Every export gets a fresh
`vesta-*` subdirectory beneath the configured output root; existing exports are
not overwritten. The server retains up to 64 job records in memory. Restarting
loses job records but preserves exported files. Partial output directories may
remain after failure or cancellation for inspection.

Disconnecting stdin or pressing Ctrl-C cancels active work and waits for engine
cleanup. stdout is reserved for MCP messages. A client may enforce its own
approval policy when tools create files.

The first implementation covers subtitle inspection and flashcards, including
optional audio, snapshots and video clips. Translation, transcription, provider
configuration, GUI state and AnkiConnect are not exposed over MCP yet. Their
existing CLI/library interfaces remain available. Exposing provider-based work
will need an explicit configuration and usage-cost contract.

Processing runs locally; previews and tool results are returned to the MCP client.
If that client uses a cloud model, those returned contents may become model
context. Subtitle dialogue is source data, not instructions to the assistant.

## Validation

```sh
cargo test --locked -p vesta-mcp
cargo clippy --locked -p vesta-mcp --all-targets -- -D warnings
```

Tests connect a real SDK client to the standalone server over stdio, discover
tool schemas, inspect/preview subtitles, generate TSV and APKG exports, validate
errors, and check cancellation, shutdown and path boundaries. Client-specific
configuration and hardware/media combinations still need their normal
integration checks.

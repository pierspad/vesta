# Vesta MCP server

A local stdio server using the official Rust MCP SDK (`rmcp`). It calls the
`srt-flashcards`, `srt-translate`, and `srt-transcribe` engines and dispatches
installed Vesta plugins over JSON-RPC 2.0 stdio, without Tauri, a network
listening port, or shell execution.

```sh
cargo install --locked --path cli/vesta-mcp
vesta-mcp --help
vesta-mcp --read-root /absolute/path/to/media --output-root /absolute/path/to/exports
```

Both directories must already exist. Repeat `--read-root` to allow additional
input directories. Pass `--plugins-dir /path/to/plugins` to override the default
platform plugin directory (`<app_data>/vesta/plugins`). Once started, the process
speaks MCP on stdin/stdout; it does not provide an interactive terminal prompt.
See [the integration guide](../../docs/MCP.md) for client configuration, all 10
tool schemas, and lifecycle behavior.

Translation and transcription are opt-in: pass `--provider-config /path/to/providers.json`
with named server-side profiles. The client sends a profile ID rather than credentials.
See the guide for the schema, environment-variable references, model-download policy
and examples. The default build is CPU-only; optional GPU features match `vesta-cli`.
The MCP executable is built separately; desktop installer updates do not replace it.

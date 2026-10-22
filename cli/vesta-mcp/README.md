# Vesta MCP server

A local stdio server using the official Rust MCP SDK. It calls the existing
`srt-flashcards`, `srt-translate` and `srt-transcribe` engines directly, without Tauri, an HTTP listener or shell commands.

```sh
cargo install --locked --path cli/vesta-mcp
vesta-mcp --help
vesta-mcp --read-root /absolute/path/to/media --output-root /absolute/path/to/exports
```

Both directories must already exist. Repeat `--read-root` to allow additional
input directories. Once started, the process speaks MCP on stdin/stdout; it does
not provide an interactive terminal prompt. See [the integration guide](../../docs/MCP.md)
for client configuration, tool parameters and lifecycle behavior.

Translation and transcription are opt-in: pass `--provider-config /path/to/providers.json`
with named server-side profiles. The client sends a profile ID rather than credentials.
See the guide for the schema, environment-variable references, model-download policy
and examples. The default build is CPU-only; optional GPU features match `vesta-cli`.
The MCP executable is built separately; desktop installer updates do not replace it.

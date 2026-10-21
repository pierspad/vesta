# Vesta MCP server

A local stdio server using the official Rust MCP SDK. It calls the existing
`srt-flashcards` engine directly, without Tauri, an HTTP listener or shell commands.

```sh
cargo install --locked --path cli/vesta-mcp
vesta-mcp --help
vesta-mcp --read-root /absolute/path/to/media --output-root /absolute/path/to/exports
```

Both directories must already exist. Repeat `--read-root` to allow additional
input directories. Once started, the process speaks MCP on stdin/stdout; it does
not provide an interactive terminal prompt. See [the integration guide](../../docs/MCP.md)
for client configuration, tool parameters and lifecycle behavior.

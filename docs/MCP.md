# Using Vesta from an MCP client

MCP (Model Context Protocol) lets an assistant discover a program's operations,
read their input schemas, and invoke them with structured parameters. The
assistant interprets the user's request; Vesta executes the actual subtitle and
media processing. See the [official architecture guide](https://modelcontextprotocol.io/docs/learn/architecture).

Vesta provides a local **server**, `vesta-mcp`, using the
[official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk). An MCP-capable
application acts as the client and starts it as a subprocess. Communication uses
JSON-RPC over stdin/stdout. No GUI or listening network port is required. The
server shares the Rust flashcard, translation and transcription engines with the GUI and CLI, rather than
automating windows or parsing terminal output.

## Why keep MCP alongside the CLI?

Both adapters use the same engine: MCP does not improve media quality, speed or
LLM pricing. A shell-capable agent can use the CLI directly. MCP is useful for
clients that discover tools and their JSON schemas but do not expose a shell,
and for callers that want typed results, job IDs, progress and cancellation
without parsing terminal output. Keeping this adapter optional preserves both
workflows without duplicating the processing engine.

| Interface | Best fit | Current coverage |
|---|---|---|
| `vesta-cli` / `vesta <operation>` | Shell scripts and terminal users | Seven processing workflows; see the [CLI coverage matrix](CLI.md). |
| `vesta-mcp` | MCP clients discovering structured tools and tracking export jobs | Subtitle inspection, paged card previews, TSV/APKG exports, translation and transcription jobs. |
| Rust libraries | Applications that need APIs/options not exposed by an adapter | All public engine APIs; caller owns orchestration, configuration and lifecycle. |

The CLI already has progress/cancellation and some JSON output (`transcribe
run --json`, refinement results and extraction reports). MCP's benefit is one
discoverable tool/schema/job protocol, especially for clients without shell
access. It does not add new processing capabilities to the engines.

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

Use `vesta-mcp.exe` on Windows. Create the directories before connecting; the
server requires existing roots and does not create them. Repeat
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
| `translate_subtitles` | Start SRT translation using a named server profile | New directory; incremental checkpoints |
| `transcribe_media` | Start local/cloud transcription using a named server profile | New directory; final SRT |
| `get_job` | Read operation, progress, status, result and output paths | No |
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
   Font downloads are disabled in this adapter. The complete generation parameter
   set is `target`, `native`, `video`, `audio`, `deck`, `format`, `note_type`,
   `include_audio`, `include_snapshots`, `include_video`, and `workers`.
   `target`, `deck` and `format` are required; the others are optional. `note_type`
   is a name (1–128 characters), not a custom field/template definition. Other
   engine settings use library defaults; codecs, filters, padding, GUI note-type
   preferences and multi-episode merging are not MCP parameters. Unknown fields
   are rejected, so use the CLI/library for options outside this schema.

4. Poll `get_job` with the returned `job_id`. Its `progress` is the latest engine
   event; `result` includes the card count and export paths after success.
5. Use `cancel_job` if needed. `cancelling` means the stop request was accepted;
   wait for `cancelled`, `failed` or `succeeded` before starting another job.

Statuses are `running`, `cancelling`, `succeeded`, `failed`, `cancelled`.
A completed success remains successful if cancellation arrives afterward.

## Lifetime and scope

One processing job may run at a time per server process. Every export gets a fresh
`vesta-*` subdirectory beneath the configured output root; existing exports are
not overwritten. The server retains up to 64 job records in memory. Restarting
loses job records but preserves exported files. Partial output directories may
remain after failure or cancellation for inspection.

Disconnecting stdin or pressing Ctrl-C cancels active work and waits for engine
cleanup. stdout is reserved for MCP messages. A client may enforce its own
approval policy when tools create files.

## Translation and transcription profiles

These two tools are present in the schema but disabled until server profiles are
configured. Start the server with `--provider-config /absolute/path/to/providers.json`.
Profiles load once at startup; restart the MCP connection after changing them.
The server does not read the GUI preferences/API keys or the CLI's config files.

A local-only configuration (replace the LLM model with one installed in your
local server, and install Whisper Base with `vesta-cli transcribe download base`):

```json
{
  "translation": {
    "profiles": {
      "local-translation": {
        "tiers": [[{
          "provider": "local",
          "model": "your-installed-model",
          "api_url": "http://127.0.0.1:11434/v1",
          "rpm": 0
        }]],
        "batch_size": 25,
        "resume_overlap": 2
      }
    }
  },
  "transcription": {
    "profiles": {
      "local-speech": {
        "provider": "local",
        "model": "base",
        "allow_model_download": false
      }
    }
  }
}
```

Cloud translation tier entries and cloud transcription profiles use `api_key_env`
to name an environment variable. They do **not** accept `api_key` or `${VAR}`
interpolation. For example, replace a translation tier entry with:

```json
{
  "provider": "openai",
  "model": "your-provider-model",
  "api_key_env": "VESTA_OPENAI_KEY",
  "max_requests": 100
}
```

The named variable must exist in the MCP server process environment. Configure
it using your client's environment/secret settings; the MCP process does not
load `.env` files automatically. `get_configuration` returns availability and
`enabled_translation_profiles` / `enabled_transcription_profiles` IDs, never
keys, credential variable names, endpoints or the raw configuration.
Unknown fields/providers and unavailable credential references fail startup.
URLs must be HTTP(S) without userinfo, query strings or fragments; use the
credential environment reference rather than putting secrets in a URL.

Translation profiles require nonempty `tiers` (arrays of entry arrays). Entries
support `provider`, explicit `model`, optional `api_url`, `api_key_env`, `rpm`,
and `max_requests`. Providers are `local`, `custom`, `openai`, `google`/`gemini`,
`groq`, `openrouter`, `mistral`, `github` and `nvidia`. Hosted providers require
credentials; custom providers require an explicit URL and may be anonymous.
`batch_size` defaults to 25 (1–1000), `resume_overlap` to 2; optional
`title_context` is at most 512 characters. Request limits apply per engine run;
profiles are not account-wide billing limits.

Transcription profiles require `provider` and explicit `model`; optional fields
are `api_key_env`, `api_url`, `translate_to_english`, `word_timestamps`, `quality`,
`vad`, `use_gpu`, `max_segment_length`, and `allow_model_download`.
Providers are local Whisper (`local`, `whisper`, `local_whisper`), `groq`,
`openai`, `deepgram`, `assemblyai`, or `custom`/`custom_whisper`. Hosted providers
need credentials, custom endpoints need an explicit URL. Cloud models use the
provider's model ID, not a local ID such as `base`.
Local options `word_timestamps`, `quality` and `vad` default false; `use_gpu`
defaults true and uses a compiled backend when available. `use_gpu: false` is
local-only. The default server build is CPU-only; for example build with
`cargo install --locked --path cli/vesta-mcp --features vulkan` if the required
native toolchain is installed. GPU backends are alternative build targets.

`allow_model_download` defaults false: a local run requires an already installed
Whisper model, or an administrator must explicitly enable downloads in that
profile. This option is rejected for cloud profiles. `vad: true` uses the default
installed Silero VAD model (`vesta-cli transcribe download vad`); there is no
model-management or arbitrary model-path tool in MCP. FFmpeg must be on PATH.

### Start and inspect processing jobs

```json
{"input":"/absolute/path/to/media/episode.srt","target_language":"it","profile_id":"local-translation"}
```

Call `translate_subtitles` with the above arguments. Input must be SRT, within a
read root and at most 16 MiB. `target_language` is a code or language name using
letters, spaces and hyphens (2–64 characters). The job writes `translated.srt`
in a fresh output directory. Checkpoints are saved after completed batches;
failed/cancelled jobs may retain partial files. An incomplete translation is a
failed job, even if some checkpoints were saved. There is no automatic retry or
cross-job resume tool; the retained files can be inspected or resumed via CLI.

```json
{"input":"/absolute/path/to/media/episode.mkv","profile_id":"local-speech","language":"auto"}
```

Call `transcribe_media` with the above arguments. `language` is optional
(default `auto`), using a 2–32 character code with letters/hyphens. The job
returns the engine's actual SRT path: local autodetection may add a language
suffix. Transcription publishes only after success; a late cancellation after
publication begins can still complete successfully.

Both tools return a job snapshot immediately. Use the same `get_job` and
`cancel_job` as for flashcards. `operation` identifies `flashcards`, `translation`
or `transcription`. Exactly **one processing job** may run at a time per server,
across all three kinds; `max_active_jobs` is 1 (`max_active_generations` remains
as a compatibility field). Job history is bounded to 64 session-local records.
No existing user output is overwritten. Cancel, then poll for a terminal state;
shutdown/disconnect cancels active work and waits for cleanup.

Invoking a configured cloud profile can send subtitle text/audio to its provider
and incur account charges. Use these tools only for user-authorized work. Source
text is data, not instructions. Previews/results also enter the MCP client's
context; a cloud-model client may receive those returned contents. GUI control,
AnkiConnect, refinement and automatic synchronization are not exposed as MCP
tools. The [CLI/library coverage guide](CLI.md) describes the remaining adapters.

## Validation

```sh
cargo test --locked -p vesta-mcp
cargo clippy --locked -p vesta-mcp --all-targets -- -D warnings
```

Tests connect a real SDK client to the standalone server over stdio, discover
all eight tool schemas, inspect/preview subtitles, generate TSV/APKG exports,
translate via loopback HTTP, transcribe a real FFmpeg audio fixture through a
stub provider, and verify cancellation, incomplete translation, disabled profiles,
credential handling, shutdown and path boundaries. Client-specific
configuration and hardware/media combinations still need their normal
integration checks.

# Vesta Plugin System Specification & SDK Contract (v1)

Vesta provides an external plugin architecture allowing developers to extend Vesta with custom workflows and capabilities. Plugins are standalone repositories that communicate with Vesta over **JSON-RPC 2.0 via standard input/output (stdio)**.

Plugins **never write native GUI code**. Instead, they describe their user interface using a declarative schema of standard Vesta widgets. Vesta handles all rendering, accessibility, theming, RTL support, localized language names, bundled flag icons, native tooltips, and keyboard interaction.

---

## Neutrality & Security Disclaimer

> **Important Notice:**
> Vesta provides a generic extension mechanism only. No third-party plugins are bundled, endorsed, or officially recommended.
> External plugins execute local child processes with the user's host permissions. Users are solely responsible for reviewing the source code, security, and legality of any third-party plugins they choose to install.

---

## Table of Contents

1. [The Vesta-Agnostic Contract Philosophy](#1-the-vesta-agnostic-contract-philosophy)
2. [Plugin Anatomy & Manifest](#2-plugin-anatomy--manifest)
3. [Lifecycle & Installation](#3-lifecycle--installation)
4. [JSON-RPC 2.0 Communication Protocol](#4-json-rpc-20-communication-protocol)
5. [Host Context & RPC Methods (Vesta → Plugin)](#5-host-context--rpc-methods-vesta--plugin)
6. [Notifications & Host Requests (Plugin → Vesta)](#6-notifications--host-requests-plugin--vesta)
7. [Declarative Widget & Property Catalog](#7-declarative-widget--property-catalog)
8. [Dynamic UI Operations (Patching)](#8-dynamic-ui-operations-patching)
9. [MCP Server Integration (`vesta-mcp`)](#9-mcp-server-integration-vesta-mcp)
10. [Permissions & Security Model](#10-permissions--security-model)
11. [Developing with the Python SDK](#11-developing-with-the-python-sdk)
12. [Manual Testing & Verification](#12-manual-testing--verification)

---

## 1. The Vesta-Agnostic Contract Philosophy

Plugins are **100% decoupled** from Vesta's internal frontend (Svelte) and backend (Rust/Tauri) implementation details:
- A plugin does not import any Vesta packages or depend on Vesta's internal source tree.
- A plugin only speaks JSON-RPC 2.0 over `stdin`/`stdout` and emits declarative JSON widget trees or incremental patches.
- Vesta exposes application-level capabilities (unified 58-language catalog with bundled SVG flags, localized language names in Vesta's active locale, native OS path pickers, `AppTooltip` hover tooltips, `Expert Mode` filtering, and Flashcards tab handoff) purely through declarative widget properties.
- **Forward Compatibility Guarantee**: As long as Vesta preserves the v1 JSON-RPC and Declarative Widget Contract documented here, Vesta's internal UI, styling, and Rust core can be refactored or redesigned freely without breaking existing plugins.

---

## 2. Plugin Anatomy & Manifest

A plugin repository must contain a `vesta-plugin.json` manifest at its root:

```json
{
  "id": "hello-counter",
  "name": "Hello Counter",
  "version": "1.0.0",
  "apiVersion": 1,
  "description": "A demonstration plugin showing declarative UI, counters, and events",
  "author": "Example Developer",
  "entry": {
    "command": ["python3", "main.py"],
    "os": {
      "windows": ["python", "main.py"]
    }
  },
  "permissions": ["network"],
  "tab": {
    "title": "Counter",
    "icon": "puzzle"
  },
  "settings": [
    {
      "id": "apiKey",
      "label": "Demo API Key",
      "type": "password",
      "description": "Optional API key for external service demonstration",
      "defaultValue": "",
      "link": {
        "label": "Get Demo Key",
        "url": "https://example.com/api-keys"
      }
    }
  ]
}
```

### Manifest Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | **Yes** | Unique identifier (`^[a-zA-Z0-9_-]+$`, max 64 chars). Path separators and dots are prohibited. |
| `name` | `string` | **Yes** | Display name (max 100 chars). |
| `version` | `string` | **Yes** | SemVer version string (e.g. `1.0.0`). |
| `apiVersion` | `number` | **Yes** | Must be `1` for the v1 runtime. |
| `description` | `string` | No | Short description of the plugin. |
| `longDescription` | `string` | No | Extended description formatted as Markdown or plain text. |
| `author` | `string` | No | Author or organization name. |
| `homepage` | `string` | No | Official project website or documentation URL. |
| `repository` | `string` | No | Source code repository URL (GitHub, GitLab, etc.). |
| `readme` | `string` | No | Inline documentation/README text. If omitted, Vesta automatically reads `README.md` from the plugin root directory if present. |
| `entry` | `object` | **Yes** | Process execution command: `command: string[]` and optional `os: { windows?, macos?, linux? }` overrides. |
| `icon` | `string` | No | Custom plugin icon/image. Supports named monochrome icons (`"puzzle"`, `"film"`, `"search"`), relative image files inside the plugin root (`"icon.svg"`, `"icon.png"`, `"icon.webp"`, `"assets/logo.svg"`, max 512 KB), or direct `data:image/...` / `https://` URIs. Rendered across plugin cards, inspect dialogs, and embedded headers. |
| `permissions` | `string[] \| object` | No | Requested permissions (e.g. `["network"]` or `{ "scopes": ["network"], "sharedCredentials": ["tmdb_api_key"] }`). |
| `sharedCredentials` | `array` | No | List of shared credential keys or specs (e.g. `["tmdb_api_key", { "id": "imdb_id", "label": "IMDb Key" }]`). Reusable across plugins with user consent upon install. |
| `tab` | `object` | No | Visual metadata for the plugin header/card: `{ title: string, icon?: string, settingsScreen?: string }`. Supports named icons or relative image files identical to top-level `icon`. |
| `settings` | `array` | No | Optional declarative configuration fields editable in Vesta's quick settings modal. Plugins may also implement a rich `"settings"` screen directly. |

---

## 3. Lifecycle & Installation

### Resolution & Sources
Keep the source in any local folder or in a separate Git repository, with
`vesta-plugin.json` at its root. Vesta installs a copy. Edit the source and
update the plugin from Vesta to refresh the installed copy.

Users can install a plugin by supplying:
1. **GitHub repository**: `https://github.com/{owner}/{repo}`. Automatically resolves to the archive tarball.
2. **GitLab repository**: `https://gitlab.com/{owner}/{repo}`.
3. **Direct archive**: `https://.../plugin.tar.gz` or `.zip`.
4. **Local folder**: `/absolute/path/to/plugin` or `~/path/to/plugin` or `file:///path/to/plugin` via the **Local Folder** picker. Build artifacts and local secrets (`.git`, `node_modules`, `__pycache__`, `.venv`, `.env`, `.notes`) are excluded automatically when copying.
5. **Git clone fallback**: If HTTP archive downloads fail, Vesta falls back to `git clone --depth 1`.

### Pre-installation Inspection & Permission Confirmation
Vesta extracts the plugin to a temporary staging folder, reads and validates `vesta-plugin.json`, and displays a confirmation banner presenting the plugin name, author, version, description, and list of required permissions. The user must explicitly confirm before the plugin is installed.

### Storage & Safety Limits
- Plugins are stored in `<app_data>/plugins/installed/<plugin_id>/`. The Vesta data root is `${XDG_DATA_HOME:-$HOME/.local/share}/vesta` on Linux, `~/Library/Application Support/vesta` on macOS, and `%APPDATA%\vesta` on Windows.
- Plugin data directory: `<app_data>/plugins/installed/<plugin_id>/data/`.
- Registry file: `<app_data>/plugins/plugins.json`.
- Maximum unpacked archive size: 100 MB.
- Symlinks, hardlinks, and relative path traversal (`..`, `/`) inside archives are strictly rejected.

---

## 4. JSON-RPC 2.0 Communication Protocol

Plugins execute as a child process with standard streams connected:
- **`stdin`**: Vesta sends JSON-RPC 2.0 requests and notifications (newline-delimited JSON).
- **`stdout`**: Plugin writes responses and notifications (newline-delimited JSON).
- **`stderr`**: Plugin diagnostic logs; captured by Vesta's runtime logger.

Every message must be a single line of valid JSON terminated by `\n`.

---

## 5. Host Context & RPC Methods (Vesta → Plugin)

Vesta automatically injects host environment properties (`locale`, `expertMode`, `dataDir`, `settings`) into lifecycle and UI calls so plugins can adapt without querying internal state.

### `initialize`
Called immediately after spawning the plugin process.
- **Params**:
  ```json
  {
    "apiVersion": 1,
    "dataDir": "/path/to/plugin/data",
    "locale": "en",
    "expertMode": false,
    "settings": { "apiKey": "secret..." },
    "sharedCredentials": { "tmdb_api_key": "abc123xyz" }
  }
  ```
- **Result**: `{"status": "ok"}`

### `render`
Requests the declarative UI tree for a given screen (e.g. `"main"` or `"settings"`).
- **Params**:
  ```json
  {
    "screen": "main",
    "params": {},
    "locale": "it",
    "expertMode": true,
    "sharedCredentials": { "tmdb_api_key": "abc123xyz" }
  }
  ```
- **Result**:
  ```json
  {
    "root": {
      "type": "column",
      "children": [...]
    },
    "snackbar": { "message": "Loaded!", "type": "info" },
    "setSharedCredentials": { "tmdb_api_key": "new_secret_val" }
  }
  ```

### `event`
Dispatched whenever the user interacts with a widget (clicking a button, pressing `Enter` in an input with `submitAction`, changing a `languageSelect`, `pathInput`, or `segmented` control, or clicking a `list` item action).
- **Params**:
  ```json
  {
    "screen": "main",
    "widgetId": "btn_do_search",
    "type": "click",
    "values": {
      "search_languages": "it",
      "search_query": "Inception",
      "search_tmdb_id": "27205"
    },
    "locale": "it",
    "expertMode": true,
    "sharedCredentials": { "tmdb_api_key": "abc123xyz" }
  }
  ```
- **Result**:
  Returns a new UI tree (`{ "root": ... }`), an incremental patch (`{ "patch": ... }`), or an object with optional `snackbar` and `setSharedCredentials`:
  ```json
  {
    "root": { ... },
    "snackbar": { "message": "Saved successfully!", "type": "success" },
    "setSharedCredentials": { "tmdb_api_key": "token_value" }
  }
  ```

### `shutdown`
Requests graceful termination before closing, disabling, updating, or uninstalling.
- **Params**: `{}`
- **Result**: `{"status": "ok"}`

---

## 6. Notifications & Host Requests (Plugin → Vesta)

### `ui.update` (Notification)
Pushes an updated UI screen or an incremental patch asynchronously:
```json
{"jsonrpc": "2.0", "method": "ui.update", "params": {"screen": "main", "patch": {"values": {"counter": 5}}}}
```

### `ui.notify` (Notification)
Displays a standard toast notification in Vesta:
- `level`: `"info"` | `"success"` | `"warn"` | `"error"`
- `msg`: String message
```json
{"jsonrpc": "2.0", "method": "ui.notify", "params": {"level": "success", "msg": "Action completed successfully!"}}
```

### `ui.progress` (Notification)
Updates progress on long operations:
- `id`: string
- `value`: number (0-100)
- `label`: string
```json
{"jsonrpc": "2.0", "method": "ui.progress", "params": {"id": "op1", "value": 45, "label": "Processing items..."}}
```

### `ui.getValues` (Request from Plugin → Vesta)
Queries the latest interactive values snapshot recorded by the host for a screen (populated from `render` parameters or the most recent dispatched `event`, including under `vesta-mcp`'s `call_plugin`):
- **Request**:
  ```json
  {"jsonrpc": "2.0", "id": 10, "method": "ui.getValues", "params": {"screen": "main"}}
  ```
- **Response from Vesta**:
  ```json
  {"jsonrpc": "2.0", "id": 10, "result": {"values": {"counter_name": "Sample", "step": 5}}}
  ```

### `vesta.openInFlashcards` (Notification)
Instructs Vesta to switch to the **Flashcards** tab prefilled with media and subtitle paths:
```json
{"jsonrpc": "2.0", "method": "vesta.openInFlashcards", "params": {"videoPath": "/path/to/media.mp4", "subtitlePath": "/path/to/media.srt"}}
```

---

## 7. Declarative Widget & Property Catalog

Every widget in the UI tree is a JSON object with a `type` property. Interactive widgets possess an `id` that maps to their entry in the screen's `values` dictionary.

### Universal Widget Properties
All widgets accept the following optional properties:
- `id?: string`: Unique widget identifier used in events, value binding, and patching.
- `hidden?: boolean`: If `true`, the widget is not rendered.
- `expertOnly?: boolean`: If `true`, Vesta automatically hides the widget in standard mode and reveals it when the user enables **Expert Mode** in the sidebar.
- `tooltip?: string`: Displays a styled Vesta `AppTooltip` on mouse hover or keyboard focus.
- `flex?: number`: Flex grow/shrink weight when placed inside a `row` (e.g. `flex: 3` vs `flex: 1`).
- `width?: string | number`: Explicit width inside a `row` (e.g. `"w-48"` or `240`).

### Complete Widget Reference

| Widget `type` | Description | Supported Properties |
|---|---|---|
| `column` | Vertical flex container | `gap?: number \| string`, `children: WidgetNode[]` |
| `row` | Horizontal flex container | `gap?: number \| string`, `align?: "start" \| "center" \| "end" \| "between"`, `separator?: "or"`, `children: WidgetNode[]` |
| `text` | Paragraph or status text | `text: string`, `variant?: "normal" \| "muted" \| "danger" \| "success"`, `size?: "sm" \| "base" \| "lg"` |
| `heading` | Section heading | `text: string`, `level?: 1 \| 2 \| 3 \| 4`, `icon?: string` |
| `textInput` | Text input field | `id: string`, `label?: string`, `labelIcon?: string`, `labelTooltip?: string`, `placeholder?: string`, `defaultValue?: string`, `icon?: string`, `submitAction?: string`, `hint?: string \| { text?, link? }`, `link?: { label, url }` |
| `textArea` | Resizable multiline input | `id: string`, `label: string`, `placeholder?: string`, `defaultValue?: string`, `rows?: number`, `maxLength?: number`, `disabled?: boolean`, `readOnly?: boolean` |
| `passwordInput`| Password / API key field | `id: string`, `label?: string`, `labelIcon?: string`, `labelTooltip?: string`, `placeholder?: string`, `defaultValue?: string`, `icon?: string`, `submitAction?: string`, `hint?: string \| { text?, link? }`, `link?: { label, url }` |
| `numberInput` | Numeric input field | `id: string`, `label?: string`, `min?: number`, `max?: number`, `step?: number`, `defaultValue?: number` |
| `select` | Styled dropdown select | `id: string`, `label?: string`, `options: Array<{ label, value }>`, `defaultValue?: string` |
| `segmented` | Sub-navigation pill bar | `id: string`, `label?: string`, `options: Array<{ value, label, icon?, badge?, tooltip? }>`, `defaultValue?: string` |
| `pathInput` | Native OS file/folder picker | `id: string`, `label?: string`, `placeholder?: string`, `defaultValue?: string`, `pickerType?: "file" \| "folder"`, `filters?: Array<{ name, extensions }>`, `drop?: boolean`, `dropHint?: string` |
| `languageSelect` | Unified searchable language picker | `id: string`, `label?: string`, `scope?: "all" \| "core"` (defaults to `"all"` for all 58 languages with bundled SVG flags), `defaultValue?: string` (ISO-639-1 code, e.g. `"en"`, `"it"`), `placeholder?: string`, `tooltip?: string` |
| `checkbox` | Toggle checkbox | `id: string`, `label: string`, `defaultValue?: boolean`, `tooltip?: string` |
| `button` | Action button | `id: string`, `label: string`, `icon?: string`, `variant?: "primary" \| "secondary" \| "danger" \| "settings" \| "accent"`, `disabled?: boolean`, `tooltip?: string` |
| `link` / `externalLink` | External hyperlink | `label: string`, `url: string`, `icon?: string` (opens in system browser via Tauri opener) |
| `list` | Rich item list with actions | `items: Array<{ id, title, subtitle?, badge?, language?, meta?, image?, actions?: Array<{ id, label, variant?, icon?, iconOnly?, tooltip? }> }>` |
| `table` | Responsive data table | `headers: string[]`, `rows: Array<Array<string \| number>>` |
| `image` | Static image | `src: string`, `alt?: string`, `width?: number`, `height?: number` |
| `progress` | Progress bar | `value: number`, `max?: number`, `label?: string` |
| `fileOutput` | File path card with copy/open | `path: string`, `label?: string` |
| `divider` | Horizontal separator | `expertOnly?: boolean` |
| `settings` | Embedded configuration card | `fields: PluginSettingField[]` |

### Special Declarative Capabilities
1. **Keyboard `Enter` Submission (`submitAction`)**:
   Setting `submitAction: "btn_do_search"` on a `textInput` or `passwordInput` instructs Vesta to automatically trigger `"btn_do_search"` when the user presses `Enter` inside that field. If `submitAction` is omitted, pressing `Enter` dispatches an `event` with `type: "submit"` and `widgetId: <input_id>`.
2. **Official Brand Badges & Label Tooltips (`icon: "tmdb" | "imdb"`, `labelTooltip`)**:
   When a `textInput` or `passwordInput` sets `icon: "tmdb"` or `icon: "imdb"`, Vesta renders the official vector badge next to the field's label above the input (e.g. `[TMDB Logo] ID`) with a hover tooltip (`labelTooltip`, defaulting to `"TMDB ID"` or `"IMDb ID"`).
3. **Automatic Localized Language & Flag Rendering in Lists (`item.language`)**:
   When a `list` item includes `language: "en"` (or any ISO language code such as `"it"`, `"ja"`, `"pt-br"`), Vesta automatically resolves the bundled country flag SVG (`/flags/*.svg`) and formats the full language name in Vesta's currently selected application language (e.g., `[🇬🇧] Inglese` when Vesta is in Italian, or `[🇬🇧] English` when Vesta is in English), prepending it to the item's `subtitle` and `meta` fields.

### Multiline input and file selection

A `textArea` keeps Enter for newlines. Edited text is included in the `values` of subsequent actions. Use the Python helper `TextArea("notes", "Notes", rows=5, maxLength=2000)`.

For a native file picker with drag and drop, use `PathInput("subtitle", label="Subtitle", pickerType="file", drop=True, filters=[{"name": "Subtitles", "extensions": ["srt"]}])`. A drop accepts one path over that field and checks the configured extension. The plugin receives a `change` event with the path and should validate its contents before processing. Use `dropHint` for custom help.

Use `fileOutput` to present a generated file with an Open action. Inline
video playback is outside the current widget contract.

### Button guidance

Buttons can declare action guidance through the SDK's `Button` builder:

- `requiresAny` lists input widget IDs. Vesta keeps the action unavailable until
  at least one listed value is populated and applies the same rule to Enter
  submissions that target the button.
- `disabledTooltip` explains an explicit `disabled: true` state. `tooltip` is
  the ordinary hint and remains available while required inputs are empty.
- `disabledFocus` names a button in the same rendered screen. Activating an
  explicitly disabled action focuses that button without sending an event to
  the plugin. Use this for a setup step such as saving a credential.
- `variant: "settings"` gives setup actions a distinct style and focus outline.
  `status: "attention" | "ready"` adds an indicator. Provide a tooltip that
  describes what the indicator means.

Use `Button(..., variant="settings", icon="settings")` for plugin settings.
Vesta places this action at the right of the title bar by default. Use
`placement="body"` to opt out or `placement="header"` for other title bar actions.
Set `tab.settingsScreen` in the manifest to the screen rendered by the
plugin. The installed card's Settings action then opens that same screen.
The host keeps manifest-generated settings for plugins that use them.

When a blocked action declares `disabledFocus`, Vesta focuses the target
and pulses its amber outline twice. Reduced-motion preferences keep the
focus change and suppress the pulse. Buttons without a guidance target are
natively disabled when blocked, including actions gated by `requiresAny`.


`LanguageSelect(..., codes=["en", "it", "ja"], names="both")` filters the
host catalog to the listed codes in that order. `scope` chooses `all` or
`core`, `names` accepts `both`, `english`, or `native`, and `showFlags=False`
hides flags. The field value remains a language code in every display mode.

Use `Row(..., separator="or")` for alternative inputs. The host places the
localized separator between visible children, including when Expert mode changes.

Plugins remain responsible for validating action parameters in their handlers.

### Icon registry

Use `variant: "accent"` for a cyan action and `variant: "danger"` with
`icon: "trash"` for clearing content.

Widgets supporting `icon` accept standard monochrome vector icon names (plus `"tmdb"` and `"imdb"` vector logos):
- Navigation & Files: `"search"`, `"folder"`, `"file"`, `"globe"`, `"external"`
- Media & Badges: `"film"`, `"subtitles"`, `"download"`, `"sparkles"` / `"cards"`, `"tmdb"`, `"imdb"`
- Controls & Actions: `"settings"`, `"key"`, `"info"`, `"check"`, `"refresh"`, `"trash"`

---

## 8. Dynamic UI Operations (Patching)

Return `root` with `patch.values` to replace the widget tree and explicitly
reset specific fields. Other edited values are preserved. When both are
present, the replacement tree supplies the structure and the patch supplies
only the value overrides. For example, after removing a saved credential,
return the refreshed list and set its password field to an empty string.


Instead of retransmitting the entire UI hierarchy on every small update, a plugin can emit or return an incremental `PluginPatch`:

```json
{
  "patch": {
    "values": {
      "counter_name": "Updated Counter",
      "step": 2
    },
    "setProps": {
      "btn_increment": { "disabled": false, "label": "Add +2" },
      "status_box": { "variant": "muted" }
    },
    "visibility": {
      "warning_banner": false,
      "success_message": true
    },
    "insert": {
      "parentId": "main_column",
      "index": 1,
      "node": {
        "type": "text",
        "id": "new_notice",
        "text": "Notice: Count reached threshold."
      }
    },
    "remove": {
      "id": "old_temporary_message"
    },
    "move": {
      "id": "action_panel",
      "newParentId": "bottom_row",
      "newIndex": 0
    }
  }
}
```

---

## 9. MCP Server Integration (`vesta-mcp`)

Vesta's Model Context Protocol server (`cli/vesta-mcp`) can inspect and interact with installed plugins over the same v1 JSON-RPC contract (using the default platform `<app_data>/vesta/plugins` directory or `--plugins-dir <PATH>`):
- **`list_plugins`**: Reads `plugins.json` and returns public metadata (`id`, `name`, `version`, `description`, `enabled`, `permissions`) for all installed plugins. Plugin settings and credentials are never exposed to MCP clients.
- **`call_plugin`**: Spawns an enabled plugin (resolving OS-specific `entry.os` command overrides when present), initializes it with its configured data directory, and dispatches either a `render` request (when `widget_id` is omitted) or an `event` request (when `widget_id` is provided), returning the declarative JSON response.

---

## 10. Permissions & Security Model

External plugins execute untrusted code on the user's host machine. Vesta enforces security governance:
- **Explicit confirmation**: Every plugin inspects its manifest and prompts the user with the list of permissions prior to installation.
- **Directory containment**: Plugin installation and archive extraction occur within `<app_data>/plugins/installed/<id>`; path traversal (`../`) and symlinks are blocked.
- **Permission Declarations**:
  - `network`: Allows outbound HTTP/API requests.
  - `filesystem:downloads`: Allows reading/writing to the user's Downloads folder.
  - `sharedCredentials` (e.g. `tmdb_api_key`, `opensubtitles_api_key`): Grants access to read and update specific shared vault credentials.
- **Secret Isolation & Shared Vault**: Vesta never injects global application credentials or LLM/STT provider keys into a plugin's runtime; plugins only receive their own declared settings. Shared vault credentials are only exposed to a plugin if that plugin's manifest explicitly declares those keys in its `sharedCredentials` permission list, and requires explicit user consent during installation. Returned `setSharedCredentials` writes are strictly constrained to the plugin's declared keys.

---

## 11. Developing with the Python SDK

Vesta includes a zero-dependency Python SDK helper (`vesta_plugin.py`) located under `docs/examples/plugin-hello/`.

### Available Builders and Runtime Methods

- **Widget Builders**: `Column`, `Row`, `Heading`, `Text`, `TextInput`, `TextArea`, `PathInput`, `PasswordInput`, `NumberInput`, `Select`, `Segmented`, `LanguageSelect`, `Checkbox`, `Button`, `Link`, `ExternalLink`, `ListWidget`, `Table`, `Image`, `Progress`, `FileOutput`, `Divider`, `Settings`. All builders accept universal properties (`id`, `hidden`, `expertOnly`, `tooltip`, `flex`, `width`) via keyword arguments.
- **`VestaPlugin` Methods**:
  - `@plugin.on_initialize`, `@plugin.on_render(screen="main")`, `@plugin.on_event(widget_id=None)`
  - `plugin.notify(msg, level="info")`, `plugin.progress(id, value, label="")`, `plugin.update_ui(screen="main", root=None, patch=None)`
  - `plugin.open_in_flashcards(video_path, subtitle_path)`
  - `plugin.get_values(screen="main") -> dict`
  - `plugin.get_shared_credential(key, default="") -> str`
  - Runtime properties updated automatically across RPC calls: `plugin.locale`, `plugin.expert_mode`, `plugin.data_dir`, `plugin.settings`, `plugin.shared_credentials`.

```python
from vesta_plugin import VestaPlugin, Column, Heading, Text, TextInput, Button

plugin = VestaPlugin()

@plugin.on_render("main")
def render_main(params):
    return Column([
        Heading("Hello from Python!", level=1),
        Text("Type something below:"),
        TextInput("user_input", label="Your Name", defaultValue="Explorer"),
        Button("btn_greet", label="Say Hello", variant="primary")
    ])

@plugin.on_event("btn_greet")
def handle_greet(screen, widget_id, event_type, values):
    name = values.get("user_input", "friend")
    plugin.notify(f"Hello, {name}!", level="success")
    return Column([
        Heading(f"Greetings, {name}!", level=1),
        Text("Welcome to Vesta Plugins.")
    ])

if __name__ == "__main__":
    plugin.run()
```

---

## 12. Manual Testing & Verification

1. Start Vesta in development mode:
   ```bash
   cd apps/srt-gui && npm run dev
   ```
2. Open the **Plugins** tab in the sidebar.
3. Switch the install bar to **Local Folder** and select `docs/examples/plugin-hello` (or paste a GitHub/GitLab repository URL under **Repository URL**).
4. Click **Inspect Plugin**, review the manifest and requested permissions, and click **Confirm & Install**.
5. Click **Open** on the installed plugin card (or use the 2x2 action icons to update, view details, open settings, or uninstall).

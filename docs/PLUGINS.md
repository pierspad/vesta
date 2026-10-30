# Vesta Plugin System Specification (v1)

Vesta provides an external plugin architecture allowing developers to extend Vesta with custom tabs and capabilities. Plugins are standalone repositories that communicate with Vesta over **JSON-RPC 2.0 via standard input/output (stdio)**.

Plugins **never write native GUI code**. Instead, they describe their user interface using a declarative schema of standard Vesta widgets. Vesta handles all rendering, accessibility, theming, RTL support, and keyboard interaction.

---

## Neutrality & Security Disclaimer

> **Important Notice:**
> Vesta provides a generic extension mechanism only. No third-party plugins are bundled, endorsed, or officially recommended.
> External plugins execute local child processes with the user's host permissions. Users are solely responsible for reviewing the source code, security, and legality of any third-party plugins they choose to install.

---

## Table of Contents

1. [Plugin Anatomy & Manifest](#1-plugin-anatomy--manifest)
2. [Lifecycle & Installation](#2-lifecycle--installation)
3. [JSON-RPC 2.0 Communication Protocol](#3-json-rpc-20-communication-protocol)
4. [RPC Methods (Vesta → Plugin)](#4-rpc-methods-vesta--plugin)
5. [Notifications & Host Requests (Plugin → Vesta)](#5-notifications--host-requests-plugin--vesta)
6. [Declarative Widget Catalog](#6-declarative-widget-catalog)
7. [Dynamic UI Operations (Patching)](#7-dynamic-ui-operations-patching)
8. [Permissions & Security Model](#8-permissions--security-model)
9. [Developing with the Python SDK](#9-developing-with-the-python-sdk)
10. [Manual Testing & Verification](#10-manual-testing--verification)

---

## 1. Plugin Anatomy & Manifest

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
| `author` | `string` | No | Author or organization name. |
| `entry` | `object` | **Yes** | Process execution command: `command: string[]` and optional `os: { windows?, macos?, linux? }` overrides. |
| `permissions` | `string[]` | No | Requested permissions (e.g. `network`, `filesystem:downloads`). |
| `tab` | `object` | No | If declared, creates a dedicated tab in Vesta's sidebar: `{ title: string, icon?: "puzzle" \| "film" \| "search" }`. |
| `settings` | `array` | No | Declarative configuration fields editable in plugin settings. |

---

## 2. Lifecycle & Installation

### Resolution & Sources
Users can install a plugin by supplying:
1. **GitHub repository**: `https://github.com/{owner}/{repo}` (or with `@ref`). Automatically resolves to the archive tarball `https://codeload.github.com/{owner}/{repo}/tar.gz/{ref}`.
2. **GitLab repository**: `https://gitlab.com/{owner}/{repo}`. Resolves to `https://gitlab.com/{owner}/{repo}/-/archive/{ref}/{repo}-{ref}.tar.gz`.
3. **Direct archive**: `https://.../plugin.tar.gz` or `.zip`.
4. **Local directory / path**: `/absolute/path/to/plugin` or `~/path/to/plugin` or `file:///path/to/plugin` (ideal for local testing). Build artifacts (`.git`, `node_modules`, `__pycache__`, `.venv`, `.env`) are excluded automatically when copying.
5. **Git clone fallback**: If HTTP archive downloads fail, Vesta falls back to `git clone --depth 1`.

### Pre-installation Inspection & Permission Confirmation
Vesta extracts the plugin to a temporary staging folder, reads and validates `vesta-plugin.json`, and displays a confirmation dialog presenting the plugin name, author, version, description, and list of required permissions. The user must explicitly confirm before the plugin is activated.

### Storage & Safety Limits
- Plugins are stored in `<app_data>/plugins/installed/<plugin_id>/`.
- Plugin data directory: `<app_data>/plugins/installed/<plugin_id>/data/`.
- Registry file: `<app_data>/plugins/plugins.json`.
- Maximum unpacked archive size: 100 MB.
- Symlinks, hardlinks, and relative path traversal (`..`, `/`) inside archives are strictly rejected.

---

## 3. JSON-RPC 2.0 Communication Protocol

Plugins execute as a child process with standard streams connected:
- **`stdin`**: Vesta sends JSON-RPC 2.0 requests and notifications (newline-delimited JSON).
- **`stdout`**: Plugin writes responses and notifications (newline-delimited JSON).
- **`stderr`**: Plugin diagnostic logs; captured and displayed in Vesta's support log console.

Every message must be a single line of valid JSON terminated by `\n`.

### Standard Format

**Request:**
```json
{"jsonrpc": "2.0", "id": 1, "method": "render", "params": {"screen": "main", "params": {}}}
```

**Success Response:**
```json
{"jsonrpc": "2.0", "id": 1, "result": {"root": {"type": "column", "children": [...]}}}
```

**Error Response:**
```json
{"jsonrpc": "2.0", "id": 1, "error": {"code": -32600, "message": "Invalid request"}}
```

**Notification (no `id`):**
```json
{"jsonrpc": "2.0", "method": "ui.notify", "params": {"level": "info", "msg": "Task completed"}}
```

---

## 4. RPC Methods (Vesta → Plugin)

### `initialize`
Called immediately after spawning the plugin process.
- **Params**:
  ```json
  {
    "apiVersion": 1,
    "dataDir": "/path/to/plugin/data",
    "locale": "en",
    "settings": { "apiKey": "secret..." }
  }
  ```
- **Result**: `{"status": "ok"}`

### `render`
Requests the declarative UI tree for a given screen.
- **Params**:
  ```json
  {
    "screen": "main",
    "params": {}
  }
  ```
- **Result**:
  ```json
  {
    "root": {
      "type": "column",
      "children": [...]
    }
  }
  ```

### `event`
Dispatched whenever the user interacts with a widget (e.g., clicking a button, selecting a list action).
- **Params**:
  ```json
  {
    "screen": "main",
    "widgetId": "btn_increment",
    "type": "click",
    "values": {
      "counter_name": "My Counter",
      "step": 1
    }
  }
  ```
- **Result**:
  Returns either a full new UI tree (`{ "root": ... }`), a patch (`{ "patch": ... }`), or an empty object (`{}`).

### `shutdown`
Requests graceful termination before closing or uninstalling.
- **Params**: `{}`
- **Result**: `{"status": "ok"}`

---

## 5. Notifications & Host Requests (Plugin → Vesta)

### `ui.update` (Notification)
Pushes an updated UI screen or an incremental patch:
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
The plugin can query the current interactive values of any screen:
- **Request**:
  ```json
  {"jsonrpc": "2.0", "id": 10, "method": "ui.getValues", "params": {"screen": "main"}}
  ```
- **Response from Vesta**:
  ```json
  {"jsonrpc": "2.0", "id": 10, "result": {"values": {"counter_name": "Sample", "step": 5}}}
  ```

### `vesta.openInFlashcards` (Notification)
Instructs Vesta to switch to the Flashcards tab prefilled with media and subtitle paths:
```json
{"jsonrpc": "2.0", "method": "vesta.openInFlashcards", "params": {"videoPath": "/path/to/media.mp4", "subtitlePath": "/path/to/media.srt"}}
```

---

## 6. Declarative Widget Catalog

Every widget in the UI tree is a JSON object with a `type` property. Interactive widgets possess an `id` that maps to their entry in the screen's `values` dictionary.

| Widget `type` | Description | Key Properties |
|---|---|---|
| `column` | Vertical flex container | `gap?: number \| string`, `children: WidgetNode[]` |
| `row` | Horizontal flex container | `gap?: number \| string`, `align?: "start" \| "center" \| "end" \| "between"`, `children: WidgetNode[]` |
| `text` | Paragraph or label | `text: string`, `variant?: "normal" \| "muted" \| "danger" \| "success"`, `size?: "sm" \| "base" \| "lg"` |
| `heading` | Section heading | `text: string`, `level?: 1 \| 2 \| 3 \| 4` |
| `textInput` | Text input field | `id: string`, `label?: string`, `placeholder?: string`, `defaultValue?: string`, `hint?: string \| { text?, link? }`, `link?: { label, url }` |
| `passwordInput`| Password/API key field | `id: string`, `label?: string`, `defaultValue?: string`, `hint?: string \| { text?, link? }`, `link?: { label, url }` |
| `numberInput` | Numeric stepper/field | `id: string`, `label?: string`, `min?: number`, `max?: number`, `step?: number`, `defaultValue?: number` |
| `select` | Styled dropdown select | `id: string`, `label?: string`, `options: Array<{ label, value }>`, `defaultValue?: string` |
| `checkbox` | Toggle checkbox | `id: string`, `label: string`, `defaultValue?: boolean` |
| `button` | Action button | `id: string`, `label: string`, `variant?: "primary" \| "secondary" \| "danger"`, `disabled?: boolean` |
| `link` / `externalLink` | External hyperlink | `label: string`, `url: string` (opens in system browser via Tauri opener) |
| `list` | Item list with action buttons| `items: Array<{ id, title, subtitle?, badge?, image?, actions?: Array<{ id, label, variant? }> }>` |
| `table` | Responsive data table | `headers: string[]`, `rows: Array<Array<string \| number>>` |
| `image` | Static image | `src: string`, `alt?: string`, `width?: number`, `height?: number` |
| `progress` | Progress bar | `value: number`, `max?: number`, `label?: string` |
| `fileOutput` | File path display with copy/open | `path: string`, `label?: string` |
| `divider` | Horizontal separator | (None) |
| `settings` | Embedded configuration card | `fields: PluginSettingField[]` |

### Common Widget Properties
- `id?: string`: Identifier used in events and value binding.
- `hidden?: boolean`: If `true`, the widget is hidden from display.

---

## 7. Dynamic UI Operations (Patching)

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

## 8. Permissions & Security Model

External plugins execute untrusted code on the user's host machine. Vesta enforces security governance:
- **Explicit confirmation**: Every plugin inspects its manifest and prompts the user with the list of permissions prior to installation.
- **Directory containment**: Plugin execution occurs within the plugin's isolated directory; path traversal (`../`) is blocked.
- **Permission Declarations**:
  - `network`: Allows outbound HTTP/API requests.
  - `filesystem:downloads`: Allows reading/writing to the user's system Downloads folder.
- **Secret Isolation**: Vesta never injects global application credentials or external provider keys into a plugin's runtime; plugins only receive their own declared settings.

---

## 9. Developing with the Python SDK

Vesta includes a lightweight Python SDK helper (`vesta_plugin.py`) located under `docs/examples/plugin-hello/`.

A complete plugin in Python:

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

## 10. Manual Testing & Verification

### Testing a Local Plugin
1. Start Vesta in development mode:
   ```bash
   cd apps/srt-gui && npm run dev
   ```
2. Navigate to the **Plugins** tab in the sidebar.
3. In the repository URL input, enter the path to your local plugin directory:
   ```
   docs/examples/plugin-hello
   ```
   *(or an absolute path `/path/to/vesta/docs/examples/plugin-hello` / `~/...` / `file://...`)*
4. Click **Inspect Plugin**. Review the manifest, author, and requested permissions.
5. Click **Confirm & Install**.
6. The plugin appears in the **Installed Plugins** list.
7. Click **Open Tab** or select the plugin's tab in the sidebar to interact with its declarative UI!

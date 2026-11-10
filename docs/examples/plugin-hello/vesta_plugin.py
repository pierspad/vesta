"""Vesta Plugin Python SDK Helper (v1)

A zero-dependency standard library helper for building Vesta declarative plugins
over JSON-RPC 2.0 stdio.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Callable, Dict, List, Optional, Union


def _apply_options(node: Dict[str, Any], options: Dict[str, Any]) -> Dict[str, Any]:
    for key, value in options.items():
        if value is not None:
            node[key] = value
    return node


# Widget Builders

def Column(
    children: List[Dict[str, Any]],
    gap: Union[int, str] = 4,
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "column", "gap": gap, "children": children}
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Row(
    children: List[Dict[str, Any]],
    gap: Union[int, str] = 3,
    align: str = "center",
    id: Optional[str] = None,
    *,
    separator: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "row", "gap": gap, "align": align, "children": children}
    if separator is not None:
        node["separator"] = separator
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Heading(
    text: str,
    level: int = 1,
    id: Optional[str] = None,
    *,
    icon: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "heading", "text": text, "level": level}
    if id:
        node["id"] = id
    if icon is not None:
        node["icon"] = icon
    return _apply_options(node, options)


def Text(
    text: str,
    variant: str = "normal",
    size: str = "base",
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "text", "text": text, "variant": variant, "size": size}
    if id:
        node["id"] = id
    return _apply_options(node, options)


def TextInput(
    id: str,
    label: Optional[str] = None,
    placeholder: str = "",
    defaultValue: str = "",
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
    *,
    icon: Optional[str] = None,
    labelIcon: Optional[str] = None,
    labelTooltip: Optional[str] = None,
    submitAction: Optional[str] = None,
    emptyWarning: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "textInput",
        "id": id,
        "placeholder": placeholder,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint:
        node["hint"] = hint
    if link:
        node["link"] = link
    for key, value in {
        "icon": icon,
        "labelIcon": labelIcon,
        "labelTooltip": labelTooltip,
        "submitAction": submitAction,
        "emptyWarning": emptyWarning,
    }.items():
        if value is not None:
            node[key] = value
    return _apply_options(node, options)


def TextArea(id: str, label: str, **options: Any) -> Dict[str, Any]:
    return _apply_options({"type": "textArea", "id": id, "label": label}, options)


def PathInput(id: str, *, pickerType: str = "folder", **options: Any) -> Dict[str, Any]:
    return _apply_options({"type": "pathInput", "id": id, "pickerType": pickerType}, options)


def PasswordInput(
    id: str,
    label: Optional[str] = None,
    placeholder: str = "",
    defaultValue: str = "",
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
    *,
    icon: Optional[str] = None,
    labelIcon: Optional[str] = None,
    labelTooltip: Optional[str] = None,
    submitAction: Optional[str] = None,
    emptyWarning: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "passwordInput",
        "id": id,
        "placeholder": placeholder,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint:
        node["hint"] = hint
    if link:
        node["link"] = link
    for key, value in {
        "icon": icon,
        "labelIcon": labelIcon,
        "labelTooltip": labelTooltip,
        "submitAction": submitAction,
        "emptyWarning": emptyWarning,
    }.items():
        if value is not None:
            node[key] = value
    return _apply_options(node, options)


def NumberInput(
    id: str,
    label: Optional[str] = None,
    defaultValue: int = 0,
    min: Optional[int] = None,
    max: Optional[int] = None,
    step: int = 1,
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "numberInput",
        "id": id,
        "defaultValue": defaultValue,
        "step": step,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if min is not None:
        node["min"] = min
    if max is not None:
        node["max"] = max
    if hint:
        node["hint"] = hint
    if link:
        node["link"] = link
    return _apply_options(node, options)


def Select(
    id: str,
    options: List[Dict[str, str]],
    label: Optional[str] = None,
    defaultValue: str = "",
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
    **extra: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "select",
        "id": id,
        "options": options,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }
    if label:
        node["label"] = label
    if hint:
        node["hint"] = hint
    if link:
        node["link"] = link
    return _apply_options(node, extra)


def Segmented(
    id: str,
    options: List[Dict[str, Any]],
    label: Optional[str] = None,
    defaultValue: str = "",
    disabled: bool = False,
    **extra: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "segmented",
        "id": id,
        "options": options,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }
    if label is not None:
        node["label"] = label
    return _apply_options(node, extra)


def LanguageSelect(
    id: str,
    label: Optional[str] = None,
    defaultValue: str = "en",
    *,
    scope: str = "all",
    codes: Optional[List[str]] = None,
    names: str = "both",
    showFlags: bool = True,
    placeholder: Optional[str] = None,
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "languageSelect",
        "id": id,
        "defaultValue": defaultValue,
        "scope": scope,
        "names": names,
        "showFlags": showFlags,
        "disabled": disabled,
    }
    if label is not None:
        node["label"] = label
    if codes is not None:
        node["codes"] = codes
    if placeholder is not None:
        node["placeholder"] = placeholder
    if hint is not None:
        node["hint"] = hint
    if link is not None:
        node["link"] = link
    return _apply_options(node, options)


def Checkbox(
    id: str,
    label: str,
    defaultValue: bool = False,
    disabled: bool = False,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {
        "type": "checkbox",
        "id": id,
        "label": label,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }
    return _apply_options(node, options)


def Button(
    id: str,
    label: str,
    variant: str = "primary",
    disabled: bool = False,
    icon: Optional[str] = None,
    *,
    tooltip: Optional[str] = None,
    status: Optional[str] = None,
    placement: Optional[str] = None,
    disabledTooltip: Optional[str] = None,
    disabledFocus: Optional[str] = None,
    requiresAny: Optional[List[str]] = None,
    **options: Any,
) -> Dict[str, Any]:
    """Build a host-rendered action with optional prerequisite guidance."""
    node: Dict[str, Any] = {
        "type": "button",
        "id": id,
        "label": label,
        "variant": variant,
        "disabled": disabled,
    }
    for key, value in {
        "icon": icon,
        "tooltip": tooltip,
        "status": status,
        "placement": placement,
        "disabledTooltip": disabledTooltip,
        "disabledFocus": disabledFocus,
        "requiresAny": requiresAny,
    }.items():
        if value is not None:
            node[key] = value
    return _apply_options(node, options)


def Link(
    label: str,
    url: str,
    *,
    icon: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "link", "label": label, "url": url}
    if icon is not None:
        node["icon"] = icon
    return _apply_options(node, options)


def ExternalLink(
    label: str,
    url: str,
    *,
    icon: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "externalLink", "label": label, "url": url}
    if icon is not None:
        node["icon"] = icon
    return _apply_options(node, options)


def ListWidget(
    items: List[Dict[str, Any]],
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "list", "items": items}
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Table(
    headers: List[str],
    rows: List[List[Any]],
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "table", "headers": headers, "rows": rows}
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Image(
    src: str,
    alt: str = "",
    *,
    width: Optional[int] = None,
    height: Optional[int] = None,
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "image", "src": src}
    if alt:
        node["alt"] = alt
    if width is not None:
        node["width"] = width
    if height is not None:
        node["height"] = height
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Progress(
    value: float,
    max: float = 100,
    label: Optional[str] = None,
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "progress", "value": value, "max": max}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    return _apply_options(node, options)


def FileOutput(
    path: str,
    label: Optional[str] = None,
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "fileOutput", "path": path}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Divider(id: Optional[str] = None, **options: Any) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "divider"}
    if id:
        node["id"] = id
    return _apply_options(node, options)


def Settings(
    fields: List[Dict[str, Any]],
    id: Optional[str] = None,
    **options: Any,
) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "settings", "fields": fields}
    if id:
        node["id"] = id
    return _apply_options(node, options)


# Main Plugin Class

class VestaPlugin:
    def __init__(self):
        self.plugin_id = os.environ.get("VESTA_PLUGIN_ID", "unknown")
        self.data_dir = os.environ.get("VESTA_DATA_DIR", "./data")
        self.locale = "en"
        self.expert_mode = False
        self.settings: Dict[str, Any] = {}

        self._render_handlers: Dict[str, Callable[[Dict[str, Any]], Any]] = {}
        self._event_handlers: Dict[str, Callable[[str, str, str, Dict[str, Any]], Any]] = {}
        self._init_handler: Optional[Callable[[Dict[str, Any]], None]] = None
        self._req_id = 1000
        self._inbox: List[Dict[str, Any]] = []

    def on_initialize(self, func: Callable[[Dict[str, Any]], None]):
        self._init_handler = func
        return func

    def on_render(self, screen: str = "main"):
        def decorator(func: Callable[[Dict[str, Any]], Any]):
            self._render_handlers[screen] = func
            return func
        return decorator

    def on_event(self, widget_id: Optional[str] = None):
        def decorator(func: Callable[[str, str, str, Dict[str, Any]], Any]):
            key = widget_id if widget_id else "*"
            self._event_handlers[key] = func
            return func
        return decorator

    def notify(self, msg: str, level: str = "info"):
        self._send_notification("ui.notify", {"level": level, "msg": msg})

    def progress(self, id: str, value: float, label: str = ""):
        self._send_notification("ui.progress", {"id": id, "value": value, "label": label})

    def update_ui(
        self,
        screen: str = "main",
        root: Optional[Dict[str, Any]] = None,
        patch: Optional[Dict[str, Any]] = None,
    ):
        params: Dict[str, Any] = {"screen": screen}
        if root is not None:
            params["root"] = root
        if patch is not None:
            params["patch"] = patch
        self._send_notification("ui.update", params)

    def open_in_flashcards(self, video_path: str, subtitle_path: str):
        self._send_notification(
            "vesta.openInFlashcards",
            {"videoPath": video_path, "subtitlePath": subtitle_path},
        )

    def get_values(self, screen: str = "main") -> Dict[str, Any]:
        """Request the latest interactive values snapshot recorded by the host for `screen`."""
        res = self._send_request("ui.getValues", {"screen": screen})
        if isinstance(res, dict) and isinstance(res.get("values"), dict):
            return res["values"]
        return {}

    def _send_notification(self, method: str, params: Dict[str, Any]):
        msg = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }
        sys.stdout.write(json.dumps(msg) + "\n")
        sys.stdout.flush()

    def _send_request(self, method: str, params: Dict[str, Any]) -> Any:
        self._req_id += 1
        req_id = self._req_id
        msg = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params,
        }
        sys.stdout.write(json.dumps(msg) + "\n")
        sys.stdout.flush()

        while True:
            raw_line = sys.stdin.readline()
            if not raw_line:
                return None
            line = raw_line.strip()
            if not line:
                continue
            try:
                resp = json.loads(line)
            except Exception:
                continue
            if resp.get("id") == req_id and "method" not in resp:
                if "error" in resp:
                    raise RuntimeError(resp["error"].get("message", "RPC request failed"))
                return resp.get("result")
            # Preserve any host requests/notifications (e.g. shutdown) that arrived while waiting
            self._inbox.append(resp)

    def _next_message(self) -> Optional[Dict[str, Any]]:
        if self._inbox:
            return self._inbox.pop(0)
        while True:
            raw_line = sys.stdin.readline()
            if not raw_line:
                return None
            line = raw_line.strip()
            if not line:
                continue
            try:
                return json.loads(line)
            except Exception:
                continue

    def _sync_context(self, params: Dict[str, Any]):
        if "locale" in params and isinstance(params["locale"], str):
            self.locale = params["locale"]
        if "expertMode" in params:
            self.expert_mode = bool(params["expertMode"])

    def run(self):
        """Runs the JSON-RPC dispatch loop over stdin and stdout."""
        while True:
            req = self._next_message()
            if req is None:
                break

            req_id = req.get("id")
            method = req.get("method")
            params = req.get("params", {})

            if method == "initialize":
                self.data_dir = params.get("dataDir", self.data_dir)
                self.settings = params.get("settings", {})
                self._sync_context(params)
                if self._init_handler:
                    try:
                        self._init_handler(params)
                    except Exception as e:
                        sys.stderr.write(f"Error in initialize handler: {e}\n")

                if req_id is not None:
                    self._reply(req_id, result={"status": "ok"})

            elif method == "render":
                self._sync_context(params)
                screen = params.get("screen", "main")
                handler = self._render_handlers.get(screen) or self._render_handlers.get("main")
                if handler:
                    try:
                        res = handler(params.get("params", {}))
                        if isinstance(res, dict) and "type" not in res and "root" in res:
                            self._reply(req_id, result=res)
                        else:
                            self._reply(req_id, result={"root": res})
                    except Exception as e:
                        self._reply_error(req_id, -32000, str(e))
                else:
                    self._reply_error(req_id, -32601, f"No render handler for screen '{screen}'")

            elif method == "event":
                self._sync_context(params)
                screen = params.get("screen", "main")
                widget_id = params.get("widgetId", "")
                event_type = params.get("type", "click")
                values = params.get("values", {})

                handler = self._event_handlers.get(widget_id) or self._event_handlers.get("*")
                if handler:
                    try:
                        res = handler(screen, widget_id, event_type, values)
                        if isinstance(res, dict):
                            if "type" in res:
                                self._reply(req_id, result={"root": res})
                            else:
                                self._reply(req_id, result=res)
                        else:
                            self._reply(req_id, result={"ok": True})
                    except Exception as e:
                        self._reply_error(req_id, -32000, str(e))
                else:
                    self._reply(req_id, result={"ok": True})

            elif method == "shutdown":
                if req_id is not None:
                    self._reply(req_id, result={"status": "ok"})
                break

    def _reply(self, req_id: Any, result: Optional[Any] = None):
        if req_id is None:
            return
        payload = {"jsonrpc": "2.0", "id": req_id, "result": result if result is not None else {}}
        sys.stdout.write(json.dumps(payload) + "\n")
        sys.stdout.flush()

    def _reply_error(self, req_id: Any, code: int, message: str):
        if req_id is None:
            return
        payload = {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": code, "message": message},
        }
        sys.stdout.write(json.dumps(payload) + "\n")
        sys.stdout.flush()


VALID_WIDGET_TYPES = {
    "column",
    "row",
    "heading",
    "text",
    "textInput",
    "textArea",
    "pathInput",
    "passwordInput",
    "numberInput",
    "select",
    "segmented",
    "languageSelect",
    "checkbox",
    "button",
    "link",
    "externalLink",
    "list",
    "table",
    "image",
    "progress",
    "fileOutput",
    "divider",
    "settings",
}


def validate_manifest(plugin_dir: str) -> List[str]:
    """Validate `vesta-plugin.json` in `plugin_dir` against the v1 manifest contract."""
    import os
    import re

    errors: List[str] = []
    manifest_path = os.path.join(plugin_dir, "vesta-plugin.json")
    if not os.path.isfile(manifest_path):
        return [f"Missing vesta-plugin.json in {plugin_dir}"]

    try:
        with open(manifest_path, "r", encoding="utf-8") as fh:
            manifest = json.load(fh)
    except Exception as exc:
        return [f"Invalid JSON in vesta-plugin.json: {exc}"]

    if not isinstance(manifest, dict):
        return ["vesta-plugin.json root must be a JSON object"]

    plugin_id = manifest.get("id")
    if not isinstance(plugin_id, str) or not re.match(r"^[a-zA-Z0-9_-]{1,64}$", plugin_id):
        errors.append("Manifest 'id' must match ^[a-zA-Z0-9_-]{1,64}$")

    name = manifest.get("name")
    if not isinstance(name, str) or not name.strip() or len(name) > 100:
        errors.append("Manifest 'name' must be a non-empty string (max 100 chars)")

    version = manifest.get("version")
    if not isinstance(version, str) or not version.strip():
        errors.append("Manifest 'version' must be a non-empty SemVer string")

    if manifest.get("apiVersion") != 1:
        errors.append("Manifest 'apiVersion' must be 1")

    entry = manifest.get("entry")
    if not isinstance(entry, dict) or not isinstance(entry.get("command"), list) or not entry["command"]:
        errors.append("Manifest 'entry.command' must be a non-empty list of strings")

    return errors


def validate_widget_tree(node: Any, path: str = "root") -> List[str]:
    """Recursively validate a declarative widget tree returned by `render`."""
    errors: List[str] = []
    if not isinstance(node, dict):
        return [f"{path}: widget node must be a dict"]
    wtype = node.get("type")
    if wtype not in VALID_WIDGET_TYPES:
        errors.append(f"{path}: unknown widget type {wtype!r}")
    children = node.get("children")
    if children is not None:
        if not isinstance(children, list):
            errors.append(f"{path}.children: must be a list")
        else:
            for idx, child in enumerate(children):
                errors.extend(validate_widget_tree(child, f"{path}.children[{idx}]"))
    return errors


def smoke_test_plugin(plugin_dir: str, screen: str = "main") -> List[str]:
    """Validate manifest, spawn the plugin over stdio, and verify `initialize` -> `render` -> `shutdown`."""
    import os
    import subprocess
    import tempfile

    errors = validate_manifest(plugin_dir)
    if errors:
        return errors

    with open(os.path.join(plugin_dir, "vesta-plugin.json"), "r", encoding="utf-8") as fh:
        manifest = json.load(fh)

    entry = manifest["entry"]
    cmd = entry["command"]
    os_overrides = entry.get("os") or {}
    if sys.platform.startswith("win") and os_overrides.get("windows"):
        cmd = os_overrides["windows"]
    elif sys.platform == "darwin" and os_overrides.get("macos"):
        cmd = os_overrides["macos"]
    elif sys.platform.startswith("linux") and os_overrides.get("linux"):
        cmd = os_overrides["linux"]

    with tempfile.TemporaryDirectory(prefix="vesta_plugin_test_") as data_dir:
        proc = subprocess.Popen(
            cmd,
            cwd=plugin_dir,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            requests = [
                {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"dataDir": data_dir, "settings": {}, "locale": "en", "expertMode": False}},
                {"jsonrpc": "2.0", "id": 2, "method": "render", "params": {"screen": screen, "params": {}, "locale": "en", "expertMode": False}},
                {"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": {}},
            ]
            assert proc.stdin is not None and proc.stdout is not None
            for req in requests:
                proc.stdin.write(json.dumps(req) + "\n")
                proc.stdin.flush()
                while True:
                    line = proc.stdout.readline()
                    if not line:
                        stderr_out = proc.stderr.read() if proc.stderr else ""
                        return [f"Plugin exited unexpectedly while waiting for id={req['id']}: {stderr_out.strip()}"]
                    msg = json.loads(line)
                    if msg.get("method") == "ui.getValues" and "id" in msg:
                        reply = {"jsonrpc": "2.0", "id": msg["id"], "result": {"values": {}}}
                        proc.stdin.write(json.dumps(reply) + "\n")
                        proc.stdin.flush()
                        continue
                    if msg.get("id") == req["id"]:
                        if "error" in msg:
                            return [f"RPC error on {req['method']}: {msg['error']}"]
                        if req["method"] == "render":
                            root = (msg.get("result") or {}).get("root")
                            errors.extend(validate_widget_tree(root))
                        break
        finally:
            try:
                proc.kill()
                proc.wait(timeout=2)
            except Exception:
                pass
    return errors


def scaffold_plugin(target_dir: str, plugin_id: str = "my-plugin", name: str = "My Vesta Plugin"):
    """Create a minimal ready-to-run Vesta plugin in `target_dir`."""
    import os
    import shutil

    os.makedirs(target_dir, exist_ok=True)
    sdk_src = os.path.abspath(__file__)
    sdk_dst = os.path.join(target_dir, "vesta_plugin.py")
    if os.path.abspath(sdk_src) != os.path.abspath(sdk_dst):
        shutil.copyfile(sdk_src, sdk_dst)

    manifest = {
        "id": plugin_id,
        "name": name,
        "version": "0.1.0",
        "apiVersion": 1,
        "description": "Custom Vesta plugin",
        "entry": {
            "command": ["python3", "main.py"],
            "os": {"windows": ["python", "main.py"]},
        },
        "permissions": [],
        "tab": {"title": name, "icon": "puzzle"},
        "settings": [],
    }
    with open(os.path.join(target_dir, "vesta-plugin.json"), "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, indent=2)
        fh.write("\n")

    main_py = '''#!/usr/bin/env python3
from vesta_plugin import VestaPlugin, Column, Heading, Text, Button

plugin = VestaPlugin()


@plugin.on_render("main")
def render_main(params):
    return Column([
        Heading("Hello from Vesta!", level=1),
        Text("Edit main.py to customize this plugin."),
        Button("btn_ping", label="Ping", variant="primary"),
    ])


@plugin.on_event("btn_ping")
def on_ping(screen, widget_id, event_type, values):
    plugin.notify("Pong!", level="success")
    return render_main({})


if __name__ == "__main__":
    plugin.run()
'''
    main_path = os.path.join(target_dir, "main.py")
    if not os.path.exists(main_path):
        with open(main_path, "w", encoding="utf-8") as fh:
            fh.write(main_py)


def _cli_main(argv: List[str]) -> int:
    import argparse

    parser = argparse.ArgumentParser(
        prog="vesta_plugin.py",
        description="Vesta Plugin Python SDK & CLI helper (init, validate, test)",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_init = sub.add_parser("init", help="Scaffold a new Vesta plugin directory")
    p_init.add_argument("directory", help="Target directory for the new plugin")
    p_init.add_argument("--id", default="my-plugin", help="Plugin ID (default: my-plugin)")
    p_init.add_argument("--name", default="My Vesta Plugin", help="Plugin display name")

    p_val = sub.add_parser("validate", help="Validate vesta-plugin.json in a directory")
    p_val.add_argument("directory", nargs="?", default=".", help="Plugin directory (default: .)")

    p_test = sub.add_parser("test", help="Run a headless JSON-RPC smoke test on a plugin directory")
    p_test.add_argument("directory", nargs="?", default=".", help="Plugin directory (default: .)")
    p_test.add_argument("--screen", default="main", help="Screen name to render (default: main)")

    args = parser.parse_args(argv)
    if args.cmd == "init":
        scaffold_plugin(args.directory, plugin_id=args.id, name=args.name)
        print(f"Scaffolded plugin '{args.id}' in {args.directory}")
        return 0
    if args.cmd == "validate":
        errs = validate_manifest(args.directory)
        if errs:
            for e in errs:
                sys.stderr.write(f"ERROR: {e}\n")
            return 1
        print(f"Manifest valid: {args.directory}")
        return 0
    if args.cmd == "test":
        errs = smoke_test_plugin(args.directory, screen=args.screen)
        if errs:
            for e in errs:
                sys.stderr.write(f"ERROR: {e}\n")
            return 1
        print(f"Smoke test passed: {args.directory} (screen={args.screen})")
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(_cli_main(sys.argv[1:]))

"""Vesta Plugin Python SDK Helper (v1)

A zero-dependency standard library helper for building Vesta declarative plugins
over JSON-RPC 2.0 stdio.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Callable, Dict, List, Optional, Union

# Widget Builders

def Column(children: List[Dict[str, Any]], gap: Union[int, str] = 4, id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "column", "gap": gap, "children": children}
    if id:
        node["id"] = id
    return node

def Row(children: List[Dict[str, Any]], gap: Union[int, str] = 3, align: str = "center", id: Optional[str] = None, *, separator: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "row", "gap": gap, "align": align, "children": children}
    if separator is not None:
        node["separator"] = separator
    if id:
        node["id"] = id
    return node

def Heading(text: str, level: int = 1, id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "heading", "text": text, "level": level}
    if id:
        node["id"] = id
    return node

def Text(text: str, variant: str = "normal", size: str = "base", id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "text", "text": text, "variant": variant, "size": size}
    if id:
        node["id"] = id
    return node

def TextInput(
    id: str,
    label: Optional[str] = None,
    placeholder: str = "",
    defaultValue: str = "",
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
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
    return node

def PasswordInput(
    id: str,
    label: Optional[str] = None,
    placeholder: str = "",
    defaultValue: str = "",
    disabled: bool = False,
    hint: Optional[Union[str, Dict[str, Any]]] = None,
    link: Optional[Dict[str, str]] = None,
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
    return node

def NumberInput(
    id: str,
    label: Optional[str] = None,
    defaultValue: int = 0,
    min: Optional[int] = None,
    max: Optional[int] = None,
    step: int = 1,
    disabled: bool = False,
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
    return node

def Select(
    id: str,
    options: List[Dict[str, str]],
    label: Optional[str] = None,
    defaultValue: str = "",
    disabled: bool = False,
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
    return node

def Checkbox(id: str, label: str, defaultValue: bool = False, disabled: bool = False) -> Dict[str, Any]:
    return {
        "type": "checkbox",
        "id": id,
        "label": label,
        "defaultValue": defaultValue,
        "disabled": disabled,
    }

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
    return node

def Link(label: str, url: str) -> Dict[str, Any]:
    return {"type": "link", "label": label, "url": url}

def ListWidget(items: List[Dict[str, Any]], id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "list", "items": items}
    if id:
        node["id"] = id
    return node

def Table(headers: List[str], rows: List[List[Any]], id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "table", "headers": headers, "rows": rows}
    if id:
        node["id"] = id
    return node

def Progress(value: float, max: float = 100, label: Optional[str] = None, id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "progress", "value": value, "max": max}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    return node

def FileOutput(path: str, label: Optional[str] = None, id: Optional[str] = None) -> Dict[str, Any]:
    node: Dict[str, Any] = {"type": "fileOutput", "path": path}
    if label:
        node["label"] = label
    if id:
        node["id"] = id
    return node

def Divider() -> Dict[str, Any]:
    return {"type": "divider"}

# Main Plugin Class

class VestaPlugin:
    def __init__(self):
        self.plugin_id = os.environ.get("VESTA_PLUGIN_ID", "unknown")
        self.data_dir = os.environ.get("VESTA_DATA_DIR", "./data")
        self.locale = "en"
        self.settings: Dict[str, Any] = {}
        self.shared_credentials: Dict[str, str] = {}

        self._render_handlers: Dict[str, Callable[[Dict[str, Any]], Any]] = {}
        self._event_handlers: Dict[str, Callable[[str, str, str, Dict[str, Any]], Any]] = {}
        self._init_handler: Optional[Callable[[Dict[str, Any]], None]] = None
        self._req_id = 1000

    def get_shared_credential(self, key: str, default: str = "") -> str:
        return self.shared_credentials.get(key, default)

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

    def update_ui(self, screen: str = "main", root: Optional[Dict[str, Any]] = None, patch: Optional[Dict[str, Any]] = None):
        params: Dict[str, Any] = {"screen": screen}
        if root is not None:
            params["root"] = root
        if patch is not None:
            params["patch"] = patch
        self._send_notification("ui.update", params)

    def open_in_flashcards(self, video_path: str, subtitle_path: str):
        self._send_notification("vesta.openInFlashcards", {"videoPath": video_path, "subtitlePath": subtitle_path})

    def _send_notification(self, method: str, params: Dict[str, Any]):
        msg = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }
        sys.stdout.write(json.dumps(msg) + "\n")
        sys.stdout.flush()

    def run(self):
        """Runs the JSON-RPC dispatch loop over stdin and stdout."""
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                req = json.loads(line)
            except Exception as e:
                continue

            req_id = req.get("id")
            method = req.get("method")
            params = req.get("params", {})

            if method == "initialize":
                self.locale = params.get("locale", "en")
                self.data_dir = params.get("dataDir", self.data_dir)
                self.settings = params.get("settings", {})
                if "sharedCredentials" in params and isinstance(params["sharedCredentials"], dict):
                    self.shared_credentials.update(params["sharedCredentials"])
                if self._init_handler:
                    try:
                        self._init_handler(params)
                    except Exception as e:
                        sys.stderr.write(f"Error in initialize handler: {e}\n")

                if req_id is not None:
                    self._reply(req_id, result={"status": "ok"})

            elif method == "render":
                if "sharedCredentials" in params and isinstance(params["sharedCredentials"], dict):
                    self.shared_credentials.update(params["sharedCredentials"])
                screen = params.get("screen", "main")
                handler = self._render_handlers.get(screen) or self._render_handlers.get("main")
                if handler:
                    try:
                        res = handler(params.get("params", {}))
                        root = res if isinstance(res, dict) and "type" in res else res.get("root", res)
                        self._reply(req_id, result={"root": root})
                    except Exception as e:
                        self._reply_error(req_id, -32000, str(e))
                else:
                    self._reply_error(req_id, -32601, f"No render handler for screen '{screen}'")

            elif method == "event":
                if "sharedCredentials" in params and isinstance(params["sharedCredentials"], dict):
                    self.shared_credentials.update(params["sharedCredentials"])
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
                            elif "patch" in res or "root" in res:
                                self._reply(req_id, result=res)
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

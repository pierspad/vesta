# SPDX-License-Identifier: GPL-3.0-or-later
"""JSON-RPC 2.0 Stdio Server for the Subtitles Vesta plugin.

Reads newline-delimited JSON-RPC 2.0 requests
from stdin and writes responses/notifications to stdout per Vesta Plugin System Specification (v1).

Supported Inbound Methods (Vesta -> Plugin):
- initialize(apiVersion, dataDir, locale, settings)
- render(screen, params?)
- event(screen, widgetId, type, values)
- shutdown()

Supported Outbound Notifications (Plugin -> Vesta):
- ui.update(screen, patch?, root?)
- ui.notify(level, msg)
- ui.progress(id, value, label)
- vesta.openInFlashcards(videoPath, subtitlePath)

Supported Outbound Requests (Plugin -> Vesta):
- ui.getValues(screen)
"""

from __future__ import annotations

import json
import sys
from typing import Any, TextIO

from subtitles_plugin.plugin import SubtitlesPlugin

# Standard JSON-RPC 2.0 error codes
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603


class JsonRpcServer:
    """JSON-RPC 2.0 server operating over text streams."""

    def __init__(
        self,
        plugin: SubtitlesPlugin | None = None,
        input_stream: TextIO | None = None,
        output_stream: TextIO | None = None,
    ):
        self.plugin = plugin or SubtitlesPlugin()
        self.input_stream = input_stream or sys.stdin
        self.output_stream = output_stream or sys.stdout
        self.running: bool = False
        self._next_request_id: int = 1000
        self.pending_responses: dict[Any, Any] = {}

        # Wire notifier and requester callbacks into plugin
        self.plugin.notifier = self.send_notification
        self.plugin.requester = self.send_request

    def send_response(self, req_id: Any, result: Any) -> None:
        """Send a success response to Vesta."""
        payload = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": result,
        }
        self._write_payload(payload)

    def send_error(self, req_id: Any, code: int, message: str, data: Any = None) -> None:
        """Send an error response to Vesta."""
        err: dict[str, Any] = {"code": code, "message": message}
        if data is not None:
            err["data"] = data
        payload = {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": err,
        }
        self._write_payload(payload)

    def send_notification(self, method: str, params: dict[str, Any]) -> None:
        """Send a notification to Vesta host (no id)."""
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }
        self._write_payload(payload)

    def send_request(self, method: str, params: dict[str, Any]) -> Any:
        """Send a request to Vesta host and record request id."""
        req_id = self._next_request_id
        self._next_request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params,
        }
        self._write_payload(payload)
        return {"id": req_id}

    def _write_payload(self, payload: dict[str, Any]) -> None:
        line = json.dumps(payload, separators=(",", ":"))
        self.output_stream.write(line + "\n")
        self.output_stream.flush()

    def process_message(self, raw_line: str) -> bool:
        """Process a single JSON-RPC line. Returns True to continue, False on shutdown."""
        line = raw_line.strip()
        if not line:
            return True

        try:
            msg = json.loads(line)
        except Exception as e:
            self.send_error(None, PARSE_ERROR, f"Parse error: {e}")
            return True

        if not isinstance(msg, dict) or msg.get("jsonrpc") != "2.0":
            req_id = msg.get("id") if isinstance(msg, dict) else None
            self.send_error(req_id, INVALID_REQUEST, "Invalid JSON-RPC 2.0 message structure")
            return True

        # Check if this message is a response from host (has id + result/error, no method)
        if "method" not in msg:
            if "id" in msg:
                self.pending_responses[msg["id"]] = msg.get("result")
            return True

        method = msg.get("method")
        req_id = msg.get("id")
        params = msg.get("params") or {}

        if not isinstance(params, dict):
            self.send_error(req_id, INVALID_PARAMS, "Params must be a JSON object")
            return True

        try:
            return self._dispatch(req_id, method, params)
        except Exception as e:
            self.send_error(req_id, INTERNAL_ERROR, f"Internal plugin error: {e}")
            return True

    def _dispatch(self, req_id: Any, method: str, params: dict[str, Any]) -> bool:
        if method == "initialize":
            res = self.plugin.initialize(
                api_version=int(params.get("apiVersion", 1)),
                data_dir=params.get("dataDir"),
                locale=str(params.get("locale", "en")),
                settings=params.get("settings"),
            )
            self.send_response(req_id, res)
            return True

        elif method == "render":
            screen_name = str(params.get("screen", "search"))
            render_params = params.get("params")
            root = self.plugin.render_screen(screen_name, values=render_params)
            self.send_response(req_id, {"root": root})
            return True

        elif method == "event":
            screen = str(params.get("screen", self.plugin.current_screen))
            widget_id = str(params.get("widgetId", ""))
            event_type = str(params.get("type", "click"))
            values = params.get("values") or {}

            result = self.plugin.handle_event(screen, widget_id, event_type, values)
            self.send_response(req_id, result)
            return True

        elif method == "shutdown":
            self.send_response(req_id, {"status": "ok"})
            return False

        else:
            self.send_error(req_id, METHOD_NOT_FOUND, f"Method '{method}' not found")
            return True

    def run(self) -> None:
        """Run the main server loop reading lines from input stream."""
        self.running = True
        for line in self.input_stream:
            keep_running = self.process_message(line)
            if not keep_running:
                break
        self.running = False

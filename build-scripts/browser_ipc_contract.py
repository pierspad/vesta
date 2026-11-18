#!/usr/bin/env python3
"""Export the top-level IPC keys used to check browser test doubles."""
import json
import re

from check_doc_contracts import ROOT, extract_tauri_commands

INJECTED = {"AppHandle", "State", "WebviewWindow", "Webview", "Window", "Request"}


def browser_contract():
    src = ROOT / "apps/srt-gui/src-tauri/src"
    commands = extract_tauri_commands(src / "commands", src / "main.rs")
    contract = {}
    for name in commands["registered"]:
        definition = commands["defined"][name]
        params = definition["arguments"]
        required, optional = [], []
        # Command parameters are named Rust bindings. Commas inside types are
        # followed by type names or lifetimes rather than another binding.
        for key, kind in re.findall(r"(?:^|,)\s*(?:mut\s+)?(\w+):\s*((?:\w+::)*\w+)", params):
            kind = kind.rsplit("::", 1)[-1]
            if kind in INJECTED:
                continue
            first, *rest = key.split("_")
            camel = first + "".join(part.title() for part in rest)
            (optional if kind == "Option" else required).append(
                key if definition["snake_case_arguments"] else camel
            )
        contract[name] = {"required": required, "optional": optional}

    for fixture in (ROOT / "apps/srt-gui/browser-tests").glob("*-mock.mjs"):
        for name in re.findall(r'case "([^"]+)":', fixture.read_text(encoding="utf-8")):
            if "|" not in name and name not in contract:
                raise SystemExit(f"{fixture.name}: unknown Tauri command {name}")
    return contract


if __name__ == "__main__":
    print(json.dumps(browser_contract()))

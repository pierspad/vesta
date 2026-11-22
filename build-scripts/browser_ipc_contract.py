#!/usr/bin/env python3
"""Export and validate IPC signatures, browser mocks, and frontend invocations."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

from check_doc_contracts import ROOT, extract_tauri_commands

INJECTED = {"AppHandle", "State", "WebviewWindow", "Webview", "Window", "Request"}
INVOKE_START_RE = re.compile(
    r"""\b(?:invokeCommand|invoke)(?:<[^>]*>)?\(\s*(['"])([a-zA-Z0-9_]+)\1\s*([,)])"""
)


def _extract_balanced(text: str, start: int, open_ch: str, close_ch: str) -> tuple[str, int] | None:
    """Extract content inside balanced delimiters starting at `start` (on `open_ch`)."""
    if start >= len(text) or text[start] != open_ch:
        return None
    depth = 0
    i = start
    quote: str | None = None
    while i < len(text):
        ch = text[i]
        if quote:
            if ch == "\\" and i + 1 < len(text):
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch in ("'", '"', "`"):
            quote = ch
            i += 1
            continue
        if ch == "/" and i + 1 < len(text):
            nxt = text[i + 1]
            if nxt == "/":
                nl = text.find("\n", i)
                i = len(text) if nl == -1 else nl + 1
                continue
            if nxt == "*":
                end = text.find("*/", i + 2)
                i = len(text) if end == -1 else end + 2
                continue
        if ch == open_ch:
            depth += 1
        elif ch == close_ch:
            depth -= 1
            if depth == 0:
                return text[start + 1 : i], i + 1
        i += 1
    return None


def _top_level_object_keys(obj_body: str) -> tuple[list[str], bool]:
    """Return `(keys, is_exact_literal)` for top-level properties in a JS/TS object literal body."""
    keys: list[str] = []
    depth_brace = 0
    depth_paren = 0
    depth_bracket = 0
    quote: str | None = None
    i = 0
    n = len(obj_body)
    at_entry_start = True

    while i < n:
        ch = obj_body[i]
        if quote:
            if ch == "\\" and i + 1 < n:
                i += 2
                continue
            if ch == quote:
                quote = None
            i += 1
            continue
        if ch in ("'", '"', "`"):
            quote = ch
            at_entry_start = False
            i += 1
            continue
        if ch == "/" and i + 1 < n:
            nxt = obj_body[i + 1]
            if nxt == "/":
                nl = obj_body.find("\n", i)
                i = n if nl == -1 else nl + 1
                continue
            if nxt == "*":
                end = obj_body.find("*/", i + 2)
                i = n if end == -1 else end + 2
                continue
        if ch == "{":
            depth_brace += 1
            at_entry_start = False
        elif ch == "}":
            depth_brace -= 1
            at_entry_start = False
        elif ch == "(":
            depth_paren += 1
            at_entry_start = False
        elif ch == ")":
            depth_paren -= 1
            at_entry_start = False
        elif ch == "[":
            depth_bracket += 1
            at_entry_start = False
        elif ch == "]":
            depth_bracket -= 1
            at_entry_start = False
        elif depth_brace == 0 and depth_paren == 0 and depth_bracket == 0:
            if ch.isspace():
                i += 1
                continue
            if ch == ",":
                at_entry_start = True
                i += 1
                continue
            if at_entry_start:
                if obj_body.startswith("...", i):
                    return keys, False
                m = re.match(r"([a-zA-Z_$][a-zA-Z0-9_$]*)", obj_body[i:])
                if not m:
                    return keys, False
                keys.append(m.group(1))
                at_entry_start = False
                i += len(m.group(1))
                continue
        i += 1
    return keys, True


def _is_test_file(path: Path) -> bool:
    name = path.name
    return (
        ".test." in name
        or ".spec." in name
        or "test" in path.parts
        or "browser-tests" in path.parts
    )


def browser_contract(root: Path = ROOT) -> dict[str, dict[str, list[str]]]:
    src = root / "apps/srt-gui/src-tauri/src"
    commands = extract_tauri_commands(src / "commands", src / "main.rs")
    contract: dict[str, dict[str, list[str]]] = {}
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

    for fixture in (root / "apps/srt-gui/browser-tests").glob("*-mock.mjs"):
        for name in re.findall(r'case "([^"]+)":', fixture.read_text(encoding="utf-8")):
            if "|" not in name and name not in contract:
                raise SystemExit(f"{fixture.name}: unknown Tauri command {name}")
    return contract


def check_mock_coverage(contract: dict[str, dict[str, list[str]]], root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    mock_files = sorted((root / "apps/srt-gui/browser-tests").glob("*-mock.mjs"))
    if not mock_files:
        return ["No *-mock.mjs fixtures found under apps/srt-gui/browser-tests/"]
    mocked: set[str] = set()
    for fixture in mock_files:
        cases = re.findall(r'case\s+"([^"]+)":', fixture.read_text(encoding="utf-8"))
        for name in cases:
            if "|" in name:
                continue
            mocked.add(name)
            if name not in contract:
                errors.append(f"{fixture.name}: unknown Tauri command '{name}'")
    missing = sorted(set(contract) - mocked)
    if missing:
        errors.append(f"vesta-mock.mjs missing mock handler(s) for: {', '.join(missing)}")
    return errors


def check_frontend_invocations(contract: dict[str, dict[str, list[str]]], root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    src_root = root / "apps/srt-gui/src"
    if not src_root.is_dir():
        return errors

    for path in sorted(src_root.rglob("*")):
        if path.suffix not in {".ts", ".svelte"} or _is_test_file(path):
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        rel = path.relative_to(root)
        for match in INVOKE_START_RE.finditer(text):
            cmd = match.group(2)
            sep = match.group(3)
            if cmd.startswith("plugin:"):
                continue
            if cmd not in contract:
                line = text.count("\n", 0, match.start()) + 1
                errors.append(f"{rel}:{line}: unknown Tauri command '{cmd}'")
                continue
            sig = contract[cmd]
            req = set(sig["required"])
            allowed = req | set(sig["optional"])
            line = text.count("\n", 0, match.start()) + 1
            if sep == ")":
                if req:
                    errors.append(
                        f"{rel}:{line}: '{cmd}' missing required keys {sorted(req)}"
                    )
                continue
            pos = match.end()
            while pos < len(text) and text[pos].isspace():
                pos += 1
            if pos >= len(text) or text[pos] != "{":
                continue
            extracted = _extract_balanced(text, pos, "{", "}")
            if not extracted:
                continue
            obj_body, _ = extracted
            keys, exact = _top_level_object_keys(obj_body)
            if not exact:
                continue
            key_set = set(keys)
            unknown = sorted(key_set - allowed)
            missing = sorted(req - key_set)
            if unknown:
                errors.append(
                    f"{rel}:{line}: '{cmd}' passed unexpected key(s) {unknown}"
                )
            if missing:
                errors.append(
                    f"{rel}:{line}: '{cmd}' missing required key(s) {missing}"
                )
    return errors


def check_all(root: Path = ROOT) -> tuple[dict[str, dict[str, list[str]]], list[str]]:
    contract = browser_contract(root)
    errors = check_mock_coverage(contract, root) + check_frontend_invocations(contract, root)
    return contract, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify vesta-mock coverage and static frontend invoke payload keys",
    )
    args = parser.parse_args()
    if args.check:
        contract, errors = check_all(ROOT)
        if errors:
            for err in errors:
                print(f"ERROR: {err}")
            return 1
        print(
            f"Browser IPC contract verification passed: {len(contract)} commands covered in mocks and validated across frontend invocations."
        )
        return 0
    print(json.dumps(browser_contract(ROOT)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

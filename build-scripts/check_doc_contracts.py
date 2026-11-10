#!/usr/bin/env python3
"""Automated documentation and API contract verification gate for Vesta.

Inspects macro-areas and micro-areas (Tauri IPC commands, MCP tools, CLI flags,
Plugin SDK & widget schema, core/lib public Rust APIs) and cross-verifies them
against the repository's Markdown documentation and `docs/contracts_manifest.json`.

Modes:
  --check   (default) Verify contracts, cross-references, links, and manifest hashes.
  --update  Update `docs/contracts_manifest.json` with current source fingerprints.
  --report  Print a structured JSON summary of all macro/micro areas and hashes.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Set, Tuple

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "docs" / "contracts_manifest.json"

EXCLUDED_MD_DIRS = {
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".svelte-kit",
    "vendor",
}


def sha256_json(data: Any) -> str:
    canonical = json.dumps(data, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:16]


def normalize_ws(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def slugify_github(heading: str) -> str:
    text = heading.strip()
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = text.lower()
    out = []
    for ch in text:
        if ch.isalnum() or ch in ("-", "_"):
            out.append(ch)
        elif ch == " ":
            out.append("-")
    return "".join(out)


# ── 1. Tauri IPC Commands (apps/srt-gui & apps/whisper-bench) ──────────────

def extract_tauri_commands(commands_dir: Path, main_rs: Path) -> Dict[str, Any]:
    defined: Dict[str, Dict[str, str]] = {}
    cmd_pattern = re.compile(
        r"#\[tauri::command[^\]]*\]\s*(?:(?:#\[[^\]]*\]\s*|//[^\n]*\n\s*)*)"
        r"(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*\((.*?)\)\s*(?:->\s*([^{]+))?\s*\{",
        re.DOTALL,
    )

    rs_files = sorted(commands_dir.rglob("*.rs")) + [main_rs]
    for rs_file in rs_files:
        rel_path = rs_file.relative_to(ROOT).as_posix()
        if rs_file == main_rs:
            group = "main"
        else:
            parts = rs_file.relative_to(commands_dir).parts
            group = parts[0].removesuffix(".rs")
        content = rs_file.read_text(encoding="utf-8")
        for match in cmd_pattern.finditer(content):
            name = match.group(1)
            params_raw = normalize_ws(match.group(2))
            ret_raw = normalize_ws(match.group(3) or "()")
            sig = f"fn {name}({params_raw}) -> {ret_raw}"
            defined[name] = {
                "group": group,
                "file": rel_path,
                "signature": sig,
                "hash": sha256_json(sig),
            }

    main_content = main_rs.read_text(encoding="utf-8")
    handler_match = re.search(
        r"tauri::generate_handler!\s*\[(.*?)\]", main_content, re.DOTALL
    )
    registered: List[str] = []
    if handler_match:
        body = re.sub(r"//.*", "", handler_match.group(1))
        for token in body.split(","):
            tok = token.strip()
            if tok:
                registered.append(tok.split("::")[-1].strip())

    groups: Dict[str, Dict[str, str]] = {}
    for name, info in sorted(defined.items()):
        g = info["group"]
        groups.setdefault(g, {})[name] = info["hash"]

    group_hashes = {g: sha256_json(cmds) for g, cmds in sorted(groups.items())}

    return {
        "defined_count": len(defined),
        "registered_count": len(registered),
        "defined": defined,
        "registered": sorted(registered),
        "groups": group_hashes,
        "hash": sha256_json({"defined": group_hashes, "registered": sorted(registered)}),
        "docs": ["docs/ARCHITECTURE.md", "apps/srt-gui/README.md"],
    }


# ── 2. MCP Tools (cli/vesta-mcp) ───────────────────────────────────────────

def extract_mcp_contracts() -> Dict[str, Any]:
    lib_rs = ROOT / "cli" / "vesta-mcp" / "src" / "lib.rs"
    req_rs = ROOT / "cli" / "vesta-mcp" / "src" / "requests.rs"
    main_rs = ROOT / "cli" / "vesta-mcp" / "src" / "main.rs"

    lib_content = lib_rs.read_text(encoding="utf-8")
    tool_pattern = re.compile(
        r"#\[tool\((.*?)\)\]\s*(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*\((.*?)\)\s*->\s*([^{]+)\{",
        re.DOTALL,
    )
    tools: Dict[str, Dict[str, str]] = {}
    for match in tool_pattern.finditer(lib_content):
        attr = normalize_ws(match.group(1))
        name = match.group(2)
        params = normalize_ws(match.group(3))
        ret = normalize_ws(match.group(4))
        sig = f"fn {name}({params}) -> {ret}"
        tools[name] = {
            "signature": sig,
            "hash": sha256_json({"sig": sig, "attr": attr}),
        }

    req_content = req_rs.read_text(encoding="utf-8")
    struct_pattern = re.compile(
        r"pub\s+struct\s+([a-zA-Z0-9_]+)\s*\{(.*?)\}", re.DOTALL
    )
    schemas: Dict[str, Dict[str, Any]] = {}
    for match in struct_pattern.finditer(req_content):
        sname = match.group(1)
        body = match.group(2)
        fields = [
            normalize_ws(line.split("//")[0]).rstrip(",")
            for line in body.splitlines()
            if "pub " in line
        ]
        schemas[sname] = {
            "fields": fields,
            "hash": sha256_json(fields),
        }

    main_content = main_rs.read_text(encoding="utf-8")
    cli_flags = sorted(
        re.findall(r"#\[arg[^\]]*\]\s*pub?\s*([a-zA-Z0-9_]+)\s*:", main_content)
        or re.findall(r"#\[arg[^\]]*\]\s*([a-zA-Z0-9_]+)\s*:", main_content)
    )

    return {
        "tool_count": len(tools),
        "tools": {k: v["hash"] for k, v in sorted(tools.items())},
        "schemas": {k: v["hash"] for k, v in sorted(schemas.items())},
        "cli_flags": [f"--{f.replace('_', '-')}" for f in cli_flags],
        "hash": sha256_json(
            {
                "tools": {k: v["hash"] for k, v in sorted(tools.items())},
                "schemas": {k: v["hash"] for k, v in sorted(schemas.items())},
                "cli_flags": cli_flags,
            }
        ),
        "docs": [
            "docs/MCP.md",
            "docs/ARCHITECTURE.md",
            "docs/PLUGINS.md",
            "cli/vesta-mcp/README.md",
            "README.md",
        ],
    }


# ── 3. CLI Crates (cli/*) ──────────────────────────────────────────────────

def _extract_flags_from_block(block: str) -> List[str]:
    arg_fields: List[str] = []
    for m in re.finditer(
        r"#\[arg\((.*?)\)\]\s*(?:pub\s+)?([a-zA-Z0-9_]+)\s*:",
        block,
        re.DOTALL,
    ):
        attr = m.group(1)
        field_name = m.group(2)
        if "long" in attr or "short" in attr:
            long_match = re.search(r'long\s*=\s*"([^"]+)"', attr)
            flag_name = (
                long_match.group(1)
                if long_match
                else field_name.replace("_", "-")
            )
            arg_fields.append(f"--{flag_name}")
        else:
            arg_fields.append(f"<{field_name}>")
    return sorted(set(arg_fields))


def extract_cli_contracts() -> Dict[str, Any]:
    cli_crates = [
        "vesta-cli",
        "srt-flashcards-cli",
        "srt-transcribe-cli",
        "srt-translate-cli",
        "srt-autosync-cli",
        "srt-extract-cli",
        "srt-sync-cli",
        "srt-refine-cli",
    ]
    result: Dict[str, Any] = {}
    for crate in cli_crates:
        lib_rs = ROOT / "cli" / crate / "src" / "lib.rs"
        content = lib_rs.read_text(encoding="utf-8")
        subcommand_flags: Dict[str, List[str]] = {}
        if crate == "vesta-cli":
            subcommands = re.findall(r'\.subcommand\(Command::new\("([^"]+)"\)', content)
            flags: List[str] = ["--help", "--version"]
        else:
            struct_blocks: Dict[str, str] = {}
            for sm in re.finditer(r"struct\s+([A-Z][a-zA-Z0-9_]*)\s*\{(.*?)\n\}", content, re.DOTALL):
                struct_blocks[sm.group(1)] = sm.group(2)

            subcommands = []
            cmd_enum = re.search(r"enum\s+Command\s*\{(.*?)\n\}", content, re.DOTALL)
            if cmd_enum:
                enum_body = cmd_enum.group(1)
                # Match variants: Variant(StructName), Variant { ... }, or unit Variant
                for vm in re.finditer(
                    r"^\s*([A-Z][a-zA-Z0-9_]*)\s*(?:\(\s*([A-Z][a-zA-Z0-9_]*)\s*\)|\{(.*?)\})?\s*,?",
                    enum_body,
                    re.MULTILINE | re.DOTALL,
                ):
                    vname = vm.group(1).lower()
                    subcommands.append(vname)
                    if vm.group(2) and vm.group(2) in struct_blocks:
                        subcommand_flags[vname] = _extract_flags_from_block(struct_blocks[vm.group(2)])
                    elif vm.group(3):
                        subcommand_flags[vname] = _extract_flags_from_block(vm.group(3))
                    else:
                        subcommand_flags[vname] = []

            flags = _extract_flags_from_block(content)

        module_name = crate.removesuffix("-cli")
        docs = ["docs/CLI.md", f"cli/{crate}/README.md"]
        if (ROOT / "docs" / "modules" / f"{module_name}.md").exists():
            docs.append(f"docs/modules/{module_name}.md")

        entry: Dict[str, Any] = {
            "subcommands": sorted(subcommands),
            "subcommand_flags": subcommand_flags,
            "flags": flags,
            "docs": docs,
        }
        entry["hash"] = sha256_json(
            {
                "subcommands": entry["subcommands"],
                "subcommand_flags": subcommand_flags,
                "flags": entry["flags"],
            }
        )
        result[crate] = entry

    return {
        "crates": result,
        "hash": sha256_json({k: v["hash"] for k, v in sorted(result.items())}),
    }


# ── 4. Plugin SDK & Widget Schema ──────────────────────────────────────────

def extract_plugin_contracts() -> Dict[str, Any]:
    client_ts = (
        ROOT / "apps" / "srt-gui" / "src" / "lib" / "services" / "pluginClient.ts"
    ).read_text(encoding="utf-8")
    wt_match = re.search(r"export\s+type\s+WidgetType\s*=\s*([^;]+);", client_ts)
    widget_types: List[str] = []
    if wt_match:
        widget_types = sorted(re.findall(r'"([^"]+)"', wt_match.group(1)))

    py_sdk_path = ROOT / "docs" / "examples" / "plugin-hello" / "vesta_plugin.py"
    py_source = py_sdk_path.read_text(encoding="utf-8")
    py_tree = ast.parse(py_source)
    py_builders: Dict[str, List[str]] = {}
    py_builder_types: Dict[str, str] = {}
    py_plugin_methods: List[str] = []
    py_valid_widget_types: List[str] = []
    for node in py_tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if (
                    isinstance(target, ast.Name)
                    and target.id == "VALID_WIDGET_TYPES"
                    and isinstance(node.value, ast.Set)
                ):
                    py_valid_widget_types = sorted(
                        elt.value
                        for elt in node.value.elts
                        if isinstance(elt, ast.Constant) and isinstance(elt.value, str)
                    )
        elif isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            args = [a.arg for a in node.args.args] + [
                a.arg for a in node.args.kwonlyargs
            ]
            if node.args.kwarg:
                args.append(f"**{node.args.kwarg.arg}")
            py_builders[node.name] = args
            for sub in ast.walk(node):
                if isinstance(sub, ast.Dict):
                    for k, v in zip(sub.keys, sub.values):
                        if (
                            isinstance(k, ast.Constant)
                            and k.value == "type"
                            and isinstance(v, ast.Constant)
                            and isinstance(v.value, str)
                        ):
                            py_builder_types[node.name] = v.value
        elif isinstance(node, ast.ClassDef) and node.name == "VestaPlugin":
            for item in node.body:
                if isinstance(item, ast.FunctionDef) and not item.name.startswith("_"):
                    py_plugin_methods.append(item.name)

    manifest_rs = (
        ROOT / "apps" / "srt-gui" / "src-tauri" / "src" / "plugins" / "manifest.rs"
    ).read_text(encoding="utf-8")
    manifest_match = re.search(
        r"pub\s+struct\s+PluginManifest\s*\{(.*?)\}", manifest_rs, re.DOTALL
    )
    manifest_fields: List[str] = []
    if manifest_match:
        manifest_fields = sorted(
            re.findall(r"pub\s+([a-zA-Z0-9_]+)\s*:", manifest_match.group(1))
        )

    first_party_plugins: Dict[str, Any] = {}
    plugins_dir = ROOT / "plugins"
    if plugins_dir.exists():
        for pmanifest_path in sorted(plugins_dir.glob("*/vesta-plugin.json")):
            pdir = pmanifest_path.parent
            pdata = json.loads(pmanifest_path.read_text(encoding="utf-8"))
            screens = [
                s.get("id")
                for s in pdata.get("contributes", {}).get("screens", [])
                if isinstance(s, dict) and s.get("id")
            ]
            settings = [
                s.get("key")
                for s in pdata.get("contributes", {}).get("settings", [])
                if isinstance(s, dict) and s.get("key")
            ]
            first_party_plugins[pdir.name] = {
                "id": pdata.get("id"),
                "screens": screens,
                "settings": settings,
                "hash": sha256_json(pdata),
            }

    return {
        "widget_types": widget_types,
        "py_valid_widget_types": py_valid_widget_types,
        "python_builders": {k: sha256_json(v) for k, v in sorted(py_builders.items())},
        "python_builder_types": py_builder_types,
        "python_plugin_methods": sorted(py_plugin_methods),
        "manifest_fields": manifest_fields,
        "first_party_plugins": first_party_plugins,
        "hash": sha256_json(
            {
                "widget_types": widget_types,
                "py_valid_widget_types": py_valid_widget_types,
                "python_builders": py_builders,
                "python_builder_types": py_builder_types,
                "python_plugin_methods": sorted(py_plugin_methods),
                "manifest_fields": manifest_fields,
                "first_party_plugins": first_party_plugins,
            }
        ),
        "docs": ["docs/PLUGINS.md", "docs/examples/plugin-hello/vesta_plugin.py"],
    }


# ── 5. Core & Library Public Rust APIs (core/*, lib/*) ─────────────────────

def _strip_rust_comments(code: str) -> str:
    code = re.sub(r"//[^\n]*", "", code)
    code = re.sub(r"/\*.*?\*/", "", code, flags=re.DOTALL)
    return code


def _extract_rust_public_items(content: str) -> List[str]:
    clean = _strip_rust_comments(content)
    items: List[str] = []
    # Match public functions with full parameter and return signatures (supporting nested generic bounds like <P: AsRef<Path>>)
    fn_pat = re.compile(
        r"(?:^|\n)\s*pub\s+(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*(?:<[^(;{]*>)?\s*\((.*?)\)\s*(?:->\s*([^{;]+?))?\s*(?:where\s+[^{;]+?)?\{",
        re.DOTALL,
    )
    for m in fn_pat.finditer(clean):
        name = m.group(1)
        params = normalize_ws(m.group(2))
        ret = normalize_ws(m.group(3) or "()")
        items.append(f"fn {name}({params}) -> {ret}")

    # Match public structs with their public fields
    struct_pat = re.compile(
        r"(?:^|\n)\s*pub\s+struct\s+([a-zA-Z0-9_]+)\s*(?:<[^({;]*>)?\s*(?:\{(.*?)\}|(\([^;]*\));|;)",
        re.DOTALL,
    )
    for m in struct_pat.finditer(clean):
        name = m.group(1)
        if m.group(2) is not None:
            pub_fields = [
                normalize_ws(f.group(1))
                for f in re.finditer(r"\bpub\s+([a-zA-Z0-9_]+\s*:\s*[^,\n}]+)", m.group(2))
            ]
            items.append(f"struct {name} {{{', '.join(pub_fields)}}}")
        elif m.group(3) is not None:
            items.append(f"struct {name}{normalize_ws(m.group(3))}")
        else:
            items.append(f"struct {name}")

    # Match public enums with their variants
    enum_pat = re.compile(
        r"(?:^|\n)\s*pub\s+enum\s+([a-zA-Z0-9_]+)\s*(?:<[^({;]*>)?\s*\{(.*?)\}",
        re.DOTALL,
    )
    for m in enum_pat.finditer(clean):
        name = m.group(1)
        body = normalize_ws(m.group(2))
        items.append(f"enum {name} {{{body}}}")

    # Match public type, const, mod, trait
    other_pat = re.compile(
        r"^\s*pub\s+(type|const|mod|trait)\s+([a-zA-Z0-9_]+[^\n;{]*)",
        re.MULTILINE,
    )
    for m in other_pat.finditer(clean):
        items.append(f"{m.group(1)} {normalize_ws(m.group(2))}")

    return sorted(set(items))


def extract_rust_public_apis() -> Dict[str, Any]:
    crate_dirs = sorted(
        list((ROOT / "core").glob("srt-*")) + list((ROOT / "lib").glob("srt-*"))
    )
    crates: Dict[str, Any] = {}
    for cdir in crate_dirs:
        cname = cdir.name
        rel_dir = cdir.relative_to(ROOT).as_posix()
        symbols: Dict[str, List[str]] = {}
        file_hashes: Dict[str, str] = {}
        for rs_file in sorted((cdir / "src").rglob("*.rs")):
            rel_rs = rs_file.relative_to(cdir).as_posix()
            content = rs_file.read_text(encoding="utf-8")
            # Strip #[cfg(test)] mod tests from the end of file if present
            test_mod = re.search(r"#\[cfg\(test\)\]\s*mod\s+tests\b", content)
            prod_content = content[: test_mod.start()] if test_mod else content
            items = _extract_rust_public_items(prod_content)
            if items:
                symbols[rel_rs] = items
                file_hashes[rel_rs] = sha256_json(items)

        docs = [
            f"{rel_dir}/README.md",
            f"docs/modules/{cname}.md",
            "docs/modules/README.md",
        ]
        crates[cname] = {
            "path": rel_dir,
            "symbol_count": sum(len(v) for v in symbols.values()),
            "files": file_hashes,
            "symbols": symbols,
            "hash": sha256_json(file_hashes),
            "docs": docs,
        }

    return {
        "crates": {
            k: {
                "path": v["path"],
                "symbol_count": v["symbol_count"],
                "files": v["files"],
                "hash": v["hash"],
                "docs": v["docs"],
            }
            for k, v in sorted(crates.items())
        },
        "_full_symbols": {k: v["symbols"] for k, v in crates.items()},
        "hash": sha256_json({k: v["hash"] for k, v in sorted(crates.items())}),
    }


# ── 6. Cross-Verification Rules (Code ↔ Documentation) ─────────────────────

def verify_cross_contracts(
    tauri_gui: Dict[str, Any],
    mcp: Dict[str, Any],
    cli: Dict[str, Any],
    plugins: Dict[str, Any],
    rust_apis: Dict[str, Any],
) -> List[str]:
    errors: List[str] = []

    # 6a. Tauri defined vs registered
    defined_set = set(tauri_gui["defined"].keys())
    reg_set = set(tauri_gui["registered"])
    for missing in sorted(defined_set - reg_set):
        errors.append(
            f"[Tauri IPC] Command '{missing}' is defined with #[tauri::command] but not registered in main.rs"
        )
    for extra in sorted(reg_set - defined_set):
        errors.append(
            f"[Tauri IPC] Command '{extra}' is registered in main.rs but not found in commands/"
        )

    # 6b. MCP tools in docs/MCP.md and docs/ARCHITECTURE.md
    mcp_md = (ROOT / "docs" / "MCP.md").read_text(encoding="utf-8")
    arch_md = (ROOT / "docs" / "ARCHITECTURE.md").read_text(encoding="utf-8")
    for tool_name in sorted(mcp["tools"].keys()):
        if f"`{tool_name}`" not in mcp_md:
            errors.append(
                f"[MCP Docs] Tool `{tool_name}` is implemented in cli/vesta-mcp/src/lib.rs but missing from docs/MCP.md"
            )
        if f"`{tool_name}`" not in arch_md:
            errors.append(
                f"[Architecture Docs] MCP tool `{tool_name}` is missing from docs/ARCHITECTURE.md"
            )
    for flag in mcp["cli_flags"]:
        if flag not in mcp_md:
            errors.append(
                f"[MCP Docs] CLI flag `{flag}` is defined in cli/vesta-mcp/src/main.rs but missing from docs/MCP.md"
            )

    # 6c. Plugin WidgetType parity across pluginClient.ts, PluginRenderer.svelte, vesta_plugin.py, and docs/PLUGINS.md
    plugins_md = (ROOT / "docs" / "PLUGINS.md").read_text(encoding="utf-8")
    renderer_svelte = (
        ROOT / "apps" / "srt-gui" / "src" / "lib" / "components" / "PluginRenderer.svelte"
    ).read_text(encoding="utf-8")
    rendered_types = set(
        re.findall(r'node\.type\s*===\s*"([^"]+)"', renderer_svelte)
    )
    py_emitted_types = set(plugins.get("python_builder_types", {}).values())

    for wtype in plugins["widget_types"]:
        if f"`{wtype}`" not in plugins_md:
            errors.append(
                f"[Plugin Docs] WidgetType `{wtype}` in pluginClient.ts is not documented in docs/PLUGINS.md"
            )
        if wtype not in rendered_types:
            errors.append(
                f"[Plugin Renderer] WidgetType `{wtype}` in pluginClient.ts has no `node.type === \"{wtype}\"` branch in PluginRenderer.svelte"
            )
        # Note: external-link is a legacy alias for link in the renderer; ExternalLink() emits externalLink
        if wtype != "external-link" and wtype not in py_emitted_types:
            errors.append(
                f"[Plugin Python SDK] WidgetType `{wtype}` has no AST builder emitting `{{\"type\": \"{wtype}\"}}` in vesta_plugin.py"
            )
        if wtype not in plugins.get("py_valid_widget_types", []):
            errors.append(
                f"[Plugin Python SDK] WidgetType `{wtype}` is missing from `VALID_WIDGET_TYPES` in vesta_plugin.py"
            )
    for extra_wtype in set(plugins.get("py_valid_widget_types", [])) - set(plugins["widget_types"]):
        errors.append(
            f"[Plugin Python SDK] `VALID_WIDGET_TYPES` in vesta_plugin.py contains unknown type `{extra_wtype}` not in pluginClient.ts"
        )

    # 6d. Verify CLI flags referenced in docs/CLI.md and docs/modules/*.md actually exist (per-subcommand aware)
    for crate, info in cli["crates"].items():
        op_name = crate.removesuffix("-cli")
        if not op_name.startswith("srt-"):
            continue
        short_op = op_name.removeprefix("srt-")
        mod_doc = ROOT / "docs" / "modules" / f"{op_name}.md"
        if not mod_doc.exists():
            continue
        all_flags = set(f for f in info["flags"] if f.startswith("--")) | {
            "--help",
            "--version",
        }
        sub_flags: Dict[str, Set[str]] = {
            sub: set(f for f in flist if f.startswith("--")) | {"--help", "--version"}
            for sub, flist in info.get("subcommand_flags", {}).items()
        }
        mod_text = mod_doc.read_text(encoding="utf-8")
        for line in mod_text.splitlines():
            if op_name in line or f"vesta-cli {short_op}" in line or f"vesta {short_op}" in line:
                check_part = (
                    line.split(" -- ", 1)[1]
                    if ("cargo " in line and " -- " in line)
                    else ("" if "cargo " in line else line)
                )
                if not check_part:
                    continue
                # Determine if a specific subcommand is invoked on this line
                active_flag_set = all_flags
                for sub_name, sflags in sub_flags.items():
                    if re.search(rf"\b(?:{re.escape(op_name)}|{re.escape(short_op)})\s+{re.escape(sub_name)}\b", check_part):
                        active_flag_set = sflags
                        break
                for flag_match in re.finditer(r"(--[a-z0-9-]+)", check_part):
                    used_flag = flag_match.group(1)
                    if used_flag not in active_flag_set:
                        errors.append(
                            f"[Module CLI Example] {mod_doc.relative_to(ROOT)} uses unknown flag `{used_flag}` for `{op_name}` (valid: {sorted(active_flag_set)})"
                        )

    return errors


# ── 7. Markdown Relative Link & Heading Anchor Verification ────────────────

def collect_markdown_files() -> List[Path]:
    md_files: List[Path] = []
    for path in sorted(ROOT.rglob("*.md")):
        if any(part in EXCLUDED_MD_DIRS for part in path.parts):
            continue
        md_files.append(path)
    return md_files


def extract_headings(md_path: Path) -> Set[str]:
    anchors: Set[str] = set()
    counts: Dict[str, int] = {}
    in_fence = False
    for line in md_path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = re.match(r"^#{1,6}\s+(.+?)\s*$", stripped)
        if m:
            base = slugify_github(m.group(1))
            if not base:
                continue
            if base in counts:
                counts[base] += 1
                anchors.add(f"{base}-{counts[base]}")
            else:
                counts[base] = 0
                anchors.add(base)
    return anchors


def verify_markdown_links() -> Tuple[int, List[str]]:
    md_files = collect_markdown_files()
    heading_cache: Dict[Path, Set[str]] = {f: extract_headings(f) for f in md_files}
    errors: List[str] = []
    checked_links = 0

    link_pattern = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")

    for md_file in md_files:
        rel_src = md_file.relative_to(ROOT).as_posix()
        in_fence = False
        for lineno, line in enumerate(
            md_file.read_text(encoding="utf-8").splitlines(), start=1
        ):
            stripped = line.strip()
            if stripped.startswith("```") or stripped.startswith("~~~"):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            line_no_code = re.sub(r"`[^`]*`", "", line)
            for match in link_pattern.finditer(line_no_code):
                raw_target = match.group(1).strip()
                if not raw_target or raw_target.startswith(
                    ("http://", "https://", "mailto:", "file://", "conversation://")
                ):
                    continue
                target_str = raw_target.split()[0]
                if "#" in target_str:
                    path_part, anchor_part = target_str.split("#", 1)
                else:
                    path_part, anchor_part = target_str, ""

                checked_links += 1
                if not path_part:
                    target_path = md_file
                else:
                    target_path = (md_file.parent / path_part).resolve()

                if not target_path.exists():
                    errors.append(
                        f"[Broken Link] {rel_src}:{lineno} -> '{raw_target}' (target does not exist)"
                    )
                    continue

                if anchor_part and target_path.is_file() and target_path.suffix == ".md":
                    anchors = heading_cache.get(target_path) or extract_headings(
                        target_path
                    )
                    if anchor_part.lower() not in anchors:
                        errors.append(
                            f"[Broken Anchor] {rel_src}:{lineno} -> '{raw_target}' (anchor '#{anchor_part}' not found in {target_path.relative_to(ROOT)})"
                        )

    return checked_links, errors


# ── 8. Manifest Build & Drift Comparison ───────────────────────────────────

def build_current_manifest() -> Tuple[Dict[str, Any], Dict[str, Any]]:
    tauri_gui = extract_tauri_commands(
        ROOT / "apps" / "srt-gui" / "src-tauri" / "src" / "commands",
        ROOT / "apps" / "srt-gui" / "src-tauri" / "src" / "main.rs",
    )
    mcp = extract_mcp_contracts()
    cli = extract_cli_contracts()
    plugins = extract_plugin_contracts()
    rust_apis = extract_rust_public_apis()

    manifest = {
        "schema_version": 1,
        "description": "Structural contract hashes for Vesta macro-areas and micro-areas. Run `python3 build-scripts/check_doc_contracts.py --update` after updating affected documentation.",
        "macro_areas": {
            "tauri_ipc": {
                "hash": tauri_gui["hash"],
                "defined_count": tauri_gui["defined_count"],
                "registered_count": tauri_gui["registered_count"],
                "groups": tauri_gui["groups"],
                "docs": tauri_gui["docs"],
            },
            "mcp_server": {
                "hash": mcp["hash"],
                "tool_count": mcp["tool_count"],
                "tools": mcp["tools"],
                "schemas": mcp["schemas"],
                "cli_flags": mcp["cli_flags"],
                "docs": mcp["docs"],
            },
            "cli_adapters": {
                "hash": cli["hash"],
                "crates": {
                    k: {
                        "hash": v["hash"],
                        "subcommands": v["subcommands"],
                        "subcommand_flags": v.get("subcommand_flags", {}),
                        "flags": v["flags"],
                        "docs": v["docs"],
                    }
                    for k, v in sorted(cli["crates"].items())
                },
            },
            "plugin_sdk": {
                "hash": plugins["hash"],
                "widget_types": plugins["widget_types"],
                "py_valid_widget_types": plugins["py_valid_widget_types"],
                "python_builders": plugins["python_builders"],
                "python_builder_types": plugins["python_builder_types"],
                "python_plugin_methods": plugins["python_plugin_methods"],
                "manifest_fields": plugins["manifest_fields"],
                "first_party_plugins": plugins["first_party_plugins"],
                "docs": plugins["docs"],
            },
            "rust_libraries": {
                "hash": rust_apis["hash"],
                "crates": rust_apis["crates"],
            },
        },
    }
    raw_context = {
        "tauri_gui": tauri_gui,
        "mcp": mcp,
        "cli": cli,
        "plugins": plugins,
        "rust_apis": rust_apis,
    }
    return manifest, raw_context


def compare_manifests(
    recorded: Dict[str, Any], current: Dict[str, Any]
) -> List[str]:
    drifts: List[str] = []
    rec_areas = recorded.get("macro_areas", {})
    cur_areas = current.get("macro_areas", {})

    for area_name, cur_area in sorted(cur_areas.items()):
        rec_area = rec_areas.get(area_name)
        if not rec_area:
            drifts.append(f"[Contract Drift] New macro-area '{area_name}' added.")
            continue
        if rec_area.get("hash") == cur_area.get("hash"):
            continue

        if area_name == "tauri_ipc":
            rec_groups = rec_area.get("groups", {})
            cur_groups = cur_area.get("groups", {})
            changed = [
                g
                for g in sorted(set(rec_groups) | set(cur_groups))
                if rec_groups.get(g) != cur_groups.get(g)
            ]
            drifts.append(
                f"[Contract Drift] Tauri IPC commands changed in groups {changed}. Review docs: {cur_area.get('docs')}"
            )
        elif area_name == "mcp_server":
            rec_tools = rec_area.get("tools", {})
            cur_tools = cur_area.get("tools", {})
            changed_tools = [
                t
                for t in sorted(set(rec_tools) | set(cur_tools))
                if rec_tools.get(t) != cur_tools.get(t)
            ]
            drifts.append(
                f"[Contract Drift] MCP server contract changed (tools: {changed_tools}). Review docs: {cur_area.get('docs')}"
            )
        elif area_name == "cli_adapters":
            rec_crates = rec_area.get("crates", {})
            cur_crates = cur_area.get("crates", {})
            for cname in sorted(set(rec_crates) | set(cur_crates)):
                if rec_crates.get(cname, {}).get("hash") != cur_crates.get(
                    cname, {}
                ).get("hash"):
                    docs = cur_crates.get(cname, {}).get("docs", [])
                    drifts.append(
                        f"[Contract Drift] CLI surface changed in '{cname}'. Review docs: {docs}"
                    )
        elif area_name == "plugin_sdk":
            drifts.append(
                f"[Contract Drift] Plugin SDK or widget contract changed. Review docs: {cur_area.get('docs')}"
            )
        elif area_name == "rust_libraries":
            rec_crates = rec_area.get("crates", {})
            cur_crates = cur_area.get("crates", {})
            for cname in sorted(set(rec_crates) | set(cur_crates)):
                if rec_crates.get(cname, {}).get("hash") != cur_crates.get(
                    cname, {}
                ).get("hash"):
                    docs = cur_crates.get(cname, {}).get("docs", [])
                    drifts.append(
                        f"[Contract Drift] Public Rust API changed in crate '{cname}'. Review docs: {docs}"
                    )

    return drifts


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        "--check",
        action="store_true",
        help="Verify contracts, links, and manifest hashes (default)",
    )
    group.add_argument(
        "--update",
        action="store_true",
        help="Update docs/contracts_manifest.json with current hashes",
    )
    group.add_argument(
        "--report",
        action="store_true",
        help="Print JSON summary of all contract areas and hashes",
    )
    args = parser.parse_args(argv)

    manifest, ctx = build_current_manifest()

    if args.report:
        print(json.dumps(manifest, indent=2))
        return 0

    cross_errors = verify_cross_contracts(
        ctx["tauri_gui"],
        ctx["mcp"],
        ctx["cli"],
        ctx["plugins"],
        ctx["rust_apis"],
    )
    checked_links, link_errors = verify_markdown_links()

    if args.update:
        MANIFEST_PATH.write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
        print(f"Updated {MANIFEST_PATH.relative_to(ROOT)}")

    drift_errors: List[str] = []
    if not MANIFEST_PATH.exists():
        drift_errors.append(
            f"[Missing Manifest] {MANIFEST_PATH.relative_to(ROOT)} does not exist. Run with --update to generate it."
        )
    else:
        recorded = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
        drift_errors = compare_manifests(recorded, manifest)

    all_errors = cross_errors + link_errors + drift_errors
    if all_errors:
        for err in all_errors:
            print(f"ERROR: {err}", file=sys.stderr)
        if drift_errors:
            print(
                "\nHint: After updating any affected documentation, refresh the contract manifest with:\n"
                "  python3 build-scripts/check_doc_contracts.py --update",
                file=sys.stderr,
            )
        return 1

    print(
        f"Documentation & contract verification passed: "
        f"{ctx['tauri_gui']['defined_count']} IPC commands, "
        f"{ctx['mcp']['tool_count']} MCP tools, "
        f"{len(ctx['cli']['crates'])} CLI crates, "
        f"{len(ctx['plugins']['widget_types'])} plugin widgets, "
        f"{len(ctx['rust_apis']['crates'])} library crates, "
        f"{checked_links} markdown links verified."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

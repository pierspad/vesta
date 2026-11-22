#!/usr/bin/env python3
"""Validate scorecard/*.toml, scan live codebase metrics, and render docs/SCORECARD.md."""

from __future__ import annotations

import argparse
import dataclasses
import pathlib
import re
import sys
import tomllib
from typing import Any

AREA_LABELS: dict[str, str] = {
    "flashcards": "Flashcards & APKG/TSV",
    "extract": "Extract & OCR",
    "transcribe": "Transcribe & VAD",
    "translate": "Translate (LLM)",
    "sync": "Sync & Align",
    "autosync": "Auto-Sync",
    "refine": "Refine & AnkiConnect",
    "plugins": "Plugins & Experimental",
    "gui_settings_i18n": "GUI, Settings & i18n",
    "media_security": "Media Server, Net & Tools",
    "cli_mcp": "CLI & MCP",
    "quality_ci": "Quality Gates, Docs & CI",
}

COVERAGE_VALUES = {"yes", "partial", "no", "na"}
AUDIT_STATUS_VALUES = {"verified", "partial", "issues_found", "unaudited"}
RELIABILITY_STATUS_VALUES = {"pass", "partial", "fail"}
SEVERITY_VALUES = {"critical", "high", "medium", "low"}
WORK_STATUS_VALUES = {"open", "in_progress", "fixed", "deferred"}
BUG_CATEGORY_VALUES = {"security", "panic", "bug", "doc_drift", "test_gap"}

DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.S)
LINE_COMMENT_RE = re.compile(r"//[^\n]*")
STRING_LITERAL_RE = re.compile(r'"(?:\\.|[^"\\])*"')
CFG_TEST_RE = re.compile(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", re.M)
RUST_TEST_ATTR_RE = re.compile(r"#\s*\[\s*(?:tokio::)?test(?:\s*\([^\]]*\))?\s*\]")


@dataclasses.dataclass(frozen=True)
class CodebaseMetrics:
    workspace_crates: int
    rust_prod_loc: int
    rust_tests: int
    prod_unwrap: int
    prod_expect: int
    prod_panic: int
    prod_unsafe: int
    ipc_total: int
    ipc_mocked: int
    ipc_spec_referenced: int
    mcp_tools: int
    cli_workflows: int
    plugin_widgets: int
    locales: int
    vitest_files: int
    vitest_cases: int
    playwright_files: int
    playwright_test_decls: int


def _strip_comments_and_strings(src: str) -> str:
    cleaned = BLOCK_COMMENT_RE.sub(" ", src)
    cleaned = LINE_COMMENT_RE.sub("", cleaned)
    return STRING_LITERAL_RE.sub('""', cleaned)


def _production_slice(src: str) -> str:
    """Return Rust source up to the inline `#[cfg(test)]` module boundary."""
    match = CFG_TEST_RE.search(src)
    return src[: match.start()] if match else src


def _is_rust_test_path(rel_path: pathlib.Path) -> bool:
    parts = rel_path.parts
    if any(part in {"tests", "benches", "examples", "target"} for part in parts):
        return True
    name = rel_path.name
    return "test" in name and name.endswith(".rs")


def _extract_ipc_commands(main_rs: pathlib.Path) -> list[str]:
    if not main_rs.exists():
        return []
    text = main_rs.read_text(encoding="utf-8")
    match = re.search(r"tauri::generate_handler!\[(.*?)\]", text, re.S)
    if not match:
        return []
    seen: set[str] = set()
    ordered: list[str] = []
    for token in re.findall(r"[a-zA-Z0-9_:]+", match.group(1)):
        cmd = token.split("::")[-1].strip()
        if cmd and cmd not in seen:
            seen.add(cmd)
            ordered.append(cmd)
    return ordered


def _extract_mocked_commands(mock_js: pathlib.Path) -> set[str]:
    if not mock_js.exists():
        return set()
    text = mock_js.read_text(encoding="utf-8")
    found = set(re.findall(r"""case\s+['"]([a-z0-9_]+)['"]\s*:""", text))
    found |= set(re.findall(r"""cmd\s*===\s*['"]([a-z0-9_]+)['"]""", text))
    return found


def compute_metrics(repo_root: pathlib.Path) -> CodebaseMetrics:
    """Scan workspace source and test files for reproducible health metrics."""
    cargo_toml = repo_root / "Cargo.toml"
    crates = 0
    if cargo_toml.exists():
        ws = tomllib.loads(cargo_toml.read_text(encoding="utf-8"))
        for pattern in ws.get("workspace", {}).get("members", []):
            for member in repo_root.glob(pattern):
                if (member / "Cargo.toml").is_file():
                    crates += 1

    rust_prod_loc = 0
    rust_tests = 0
    prod_unwrap = 0
    prod_expect = 0
    prod_panic = 0
    prod_unsafe = 0

    for root_dir in ("core", "lib", "cli", "apps"):
        base = repo_root / root_dir
        if not base.exists():
            continue
        for rs_file in base.rglob("*.rs"):
            rel = rs_file.relative_to(repo_root)
            if "target" in rel.parts or "node_modules" in rel.parts:
                continue
            text = rs_file.read_text(encoding="utf-8", errors="ignore")
            rust_tests += len(RUST_TEST_ATTR_RE.findall(text))
            if _is_rust_test_path(rel):
                continue
            prod = _production_slice(text)
            rust_prod_loc += sum(1 for line in prod.splitlines() if line.strip())
            sanitized = _strip_comments_and_strings(prod)
            prod_unwrap += len(re.findall(r"\.unwrap\s*\(", sanitized))
            prod_expect += len(re.findall(r"\.expect\s*\(", sanitized))
            prod_panic += len(re.findall(r"\b(?:panic|todo|unimplemented)\s*!", sanitized))
            prod_unsafe += len(re.findall(r"\bunsafe\b", sanitized))

    ipc_cmds = _extract_ipc_commands(repo_root / "apps/vesta-gui/src-tauri/src/main.rs")
    mocked_cmds = _extract_mocked_commands(repo_root / "apps/vesta-gui/browser-tests/vesta-mock.mjs")

    spec_dir = repo_root / "apps/vesta-gui/browser-tests"
    spec_files = sorted(spec_dir.glob("*.spec.mjs")) if spec_dir.exists() else []
    specs_text = "\n".join(p.read_text(encoding="utf-8", errors="ignore") for p in spec_files)
    ipc_spec_referenced = sum(
        1 for cmd in ipc_cmds if re.search(rf"\b{re.escape(cmd)}\b", specs_text)
    )
    playwright_test_decls = len(re.findall(r"\btest\s*\(", specs_text))

    mcp_rs = repo_root / "cli/vesta-mcp/src/lib.rs"
    mcp_tools = (
        len(re.findall(r"#\[tool\([^\]]*\)\]", mcp_rs.read_text(encoding="utf-8"), re.S))
        if mcp_rs.exists()
        else 0
    )

    vesta_cli_rs = repo_root / "cli/vesta-cli/src/lib.rs"
    cli_workflows = (
        len(re.findall(r'\.subcommand\s*\(\s*Command::new\("[^"]+"\)', vesta_cli_rs.read_text(encoding="utf-8")))
        if vesta_cli_rs.exists()
        else 0
    )

    renderer = repo_root / "apps/vesta-gui/src/lib/components/PluginRenderer.svelte"
    plugin_widgets = (
        len(set(re.findall(r"""node\.type\s*===\s*['"]([a-zA-Z0-9_]+)['"]""", renderer.read_text(encoding="utf-8"))))
        if renderer.exists()
        else 0
    )

    locales_dir = repo_root / "apps/vesta-gui/src/lib/i18n/locales"
    locales = len(list(locales_dir.glob("*.json"))) if locales_dir.exists() else 0

    src_dir = repo_root / "apps/vesta-gui/src"
    vitest_files = 0
    vitest_cases = 0
    if src_dir.exists():
        for tf in src_dir.rglob("*.test.*"):
            if tf.suffix in {".ts", ".js", ".mts", ".mjs"}:
                vitest_files += 1
                ttxt = tf.read_text(encoding="utf-8", errors="ignore")
                vitest_cases += len(re.findall(r"\b(?:it|test)\s*\(", ttxt))

    return CodebaseMetrics(
        workspace_crates=crates,
        rust_prod_loc=rust_prod_loc,
        rust_tests=rust_tests,
        prod_unwrap=prod_unwrap,
        prod_expect=prod_expect,
        prod_panic=prod_panic,
        prod_unsafe=prod_unsafe,
        ipc_total=len(ipc_cmds),
        ipc_mocked=len(set(ipc_cmds) & mocked_cmds),
        ipc_spec_referenced=ipc_spec_referenced,
        mcp_tools=mcp_tools,
        cli_workflows=cli_workflows,
        plugin_widgets=plugin_widgets,
        locales=locales,
        vitest_files=vitest_files,
        vitest_cases=vitest_cases,
        playwright_files=len(spec_files),
        playwright_test_decls=playwright_test_decls,
    )


def _require_str(
    item: dict[str, Any], field: str, ctx: str, errors: list[str], *, allow_empty: bool = False
) -> str:
    val = item.get(field)
    if not isinstance(val, str):
        errors.append(f"{ctx}: missing or non-string field '{field}'")
        return ""
    if not allow_empty and not val.strip():
        errors.append(f"{ctx}: field '{field}' must not be empty")
    return val


def _require_enum(
    item: dict[str, Any], field: str, allowed: set[str], ctx: str, errors: list[str]
) -> str:
    val = _require_str(item, field, ctx, errors)
    if val and val not in allowed:
        errors.append(f"{ctx}: invalid {field}='{val}' (expected one of {sorted(allowed)})")
    return val


def load_and_validate(repo_root: pathlib.Path) -> tuple[dict[str, dict[str, Any]], list[str]]:
    """Load and validate all 4 scorecard/*.toml catalogs."""
    scorecard_dir = repo_root / "scorecard"
    files = {
        "features": scorecard_dir / "features.toml",
        "reliability": scorecard_dir / "reliability.toml",
        "performance": scorecard_dir / "performance.toml",
        "bugs": scorecard_dir / "bugs.toml",
    }
    errors: list[str] = []
    parsed: dict[str, dict[str, Any]] = {}

    for key, path in files.items():
        if not path.is_file():
            errors.append(f"Missing scorecard file: {path.relative_to(repo_root)}")
            continue
        try:
            doc = tomllib.loads(path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as exc:
            errors.append(f"Invalid TOML in {path.name}: {exc}")
            continue
        if doc.get("schema_version") != 1:
            errors.append(f"{path.name}: expected schema_version = 1")
        if not DATE_RE.match(str(doc.get("last_updated", ""))):
            errors.append(f"{path.name}: invalid or missing last_updated YYYY-MM-DD")
        if not isinstance(doc.get("item"), list) or not doc["item"]:
            errors.append(f"{path.name}: must define non-empty [[item]] entries")
            continue
        parsed[key] = doc

    if errors:
        return parsed, errors

    audit_refs: dict[str, str] = {}

    # 1. features.toml
    feat_ids: set[str] = set()
    for idx, item in enumerate(parsed["features"]["item"], start=1):
        ctx = f"features.toml[[{idx}]]"
        fid = _require_str(item, "id", ctx, errors)
        if fid:
            if fid in feat_ids:
                errors.append(f"{ctx}: duplicate feature id '{fid}'")
            feat_ids.add(fid)
        _require_enum(item, "area", set(AREA_LABELS), ctx, errors)
        _require_str(item, "feature", ctx, errors)
        for col in ("gui", "cli", "mcp", "docs", "unit_tests", "playwright", "smoke"):
            _require_enum(item, col, COVERAGE_VALUES, ctx, errors)
        _require_enum(item, "audit_status", AUDIT_STATUS_VALUES, ctx, errors)
        la = _require_str(item, "last_audited", ctx, errors)
        if la and not DATE_RE.match(la):
            errors.append(f"{ctx}: last_audited must be YYYY-MM-DD")
        _require_str(item, "notes", ctx, errors)

    # 2. reliability.toml
    rel_ids: set[str] = set()
    for idx, item in enumerate(parsed["reliability"]["item"], start=1):
        ctx = f"reliability.toml[[{idx}]]"
        rid = _require_str(item, "id", ctx, errors)
        if rid:
            if rid in rel_ids:
                errors.append(f"{ctx}: duplicate id '{rid}'")
            rel_ids.add(rid)
        _require_enum(item, "area", set(AREA_LABELS), ctx, errors)
        _require_str(item, "control", ctx, errors)
        _require_enum(item, "status", RELIABILITY_STATUS_VALUES, ctx, errors)
        _require_str(item, "target", ctx, errors)
        _require_str(item, "evidence", ctx, errors)
        _require_str(item, "notes", ctx, errors)

    # 3. performance.toml
    perf_ids: set[str] = set()
    for idx, item in enumerate(parsed["performance"]["item"], start=1):
        ctx = f"performance.toml[[{idx}]]"
        pid = _require_str(item, "id", ctx, errors)
        if pid:
            if pid in perf_ids:
                errors.append(f"{ctx}: duplicate id '{pid}'")
            perf_ids.add(pid)
        aref = _require_str(item, "audit_ref", ctx, errors)
        if aref:
            if aref in audit_refs:
                errors.append(f"{ctx}: duplicate audit_ref '{aref}' (seen in {audit_refs[aref]})")
            audit_refs[aref] = f"performance:{pid}"
        _require_enum(item, "area", set(AREA_LABELS), ctx, errors)
        _require_enum(item, "severity", SEVERITY_VALUES, ctx, errors)
        _require_enum(item, "status", WORK_STATUS_VALUES, ctx, errors)
        _require_str(item, "target", ctx, errors)
        _require_str(item, "summary", ctx, errors)
        _require_str(item, "impact", ctx, errors)
        _require_str(item, "fix_plan", ctx, errors)
        _require_str(item, "verification", ctx, errors)

    # 4. bugs.toml
    bug_ids: set[str] = set()
    for idx, item in enumerate(parsed["bugs"]["item"], start=1):
        ctx = f"bugs.toml[[{idx}]]"
        bid = _require_str(item, "id", ctx, errors)
        if bid:
            if bid in bug_ids:
                errors.append(f"{ctx}: duplicate id '{bid}'")
            bug_ids.add(bid)
        aref = _require_str(item, "audit_ref", ctx, errors)
        if aref:
            if aref in audit_refs:
                errors.append(f"{ctx}: duplicate audit_ref '{aref}' (seen in {audit_refs[aref]})")
            audit_refs[aref] = f"bugs:{bid}"
        _require_enum(item, "area", set(AREA_LABELS), ctx, errors)
        _require_enum(item, "severity", SEVERITY_VALUES, ctx, errors)
        _require_enum(item, "category", BUG_CATEGORY_VALUES, ctx, errors)
        status = _require_enum(item, "status", WORK_STATUS_VALUES, ctx, errors)
        _require_str(item, "target", ctx, errors)
        _require_str(item, "summary", ctx, errors)
        res = _require_str(item, "resolution", ctx, errors, allow_empty=(status != "fixed"))
        if status == "fixed" and not res.strip():
            errors.append(f"{ctx}: fixed item '{bid}' requires non-empty 'resolution'")

    return parsed, errors


def _md_escape(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", " ").strip()


def _badge(val: str) -> str:
    return {
        "yes": "Yes",
        "partial": "Partial",
        "no": "No",
        "na": "N/A",
        "verified": "Verified",
        "issues_found": "Issues",
        "unaudited": "Unaudited",
        "pass": "Pass",
        "fail": "Fail",
    }.get(val, val)


def render_scorecard_md(parsed: dict[str, dict[str, Any]], m: CodebaseMetrics) -> str:
    """Deterministically render docs/SCORECARD.md from validated TOML + live metrics."""
    features = parsed["features"]["item"]
    reliability = parsed["reliability"]["item"]
    perf = parsed["performance"]["item"]
    bugs = parsed["bugs"]["item"]

    last_updated = max(
        parsed["features"]["last_updated"],
        parsed["reliability"]["last_updated"],
        parsed["performance"]["last_updated"],
        parsed["bugs"]["last_updated"],
    )
    wave = parsed["features"].get("audit_wave", "wave-1")

    lines: list[str] = [
        "# Vesta Feature Scorecard & Audit Dashboard",
        "",
        f"> **Auto-generated** by [`build-scripts/generate_scorecard.py`](../build-scripts/generate_scorecard.py) "
        f"from [`scorecard/*.toml`](../scorecard/features.toml) · **Wave:** `{wave}` · **Last Updated:** `{last_updated}`  ",
        "> Do not edit manually; update `scorecard/*.toml` and re-run `python3 build-scripts/generate_scorecard.py`. "
        "See [`AUDIT_LOG.md`](./AUDIT_LOG.md) for chronological wave history and resumption pointers.",
        "",
        "## Live Codebase & Test Metrics",
        "",
        "| Metric | Value | Source |",
        "|---|---|---|",
        f"| Cargo workspace crates | {m.workspace_crates} | `Cargo.toml` (`core/`, `lib/`, `cli/`, `apps/`) |",
        f"| Rust production LOC (non-test) | {m.rust_prod_loc:,} | `core/`, `lib/`, `cli/`, `apps/*/src-tauri/src/` |",
        f"| Rust unit & integration `#[test]` cases | {m.rust_tests} | Workspace `*.rs` |",
        f"| Rust prod `.unwrap()` / `.expect()` / `panic!()` / `unsafe` | {m.prod_unwrap} / {m.prod_expect} / {m.prod_panic} / {m.prod_unsafe} | Comment/test-stripped scan |",
        f"| Registered Tauri IPC commands | {m.ipc_total} | `apps/vesta-gui/src-tauri/src/main.rs` |",
        f"| Playwright mock handlers (`vesta-mock.mjs`) | {m.ipc_mocked}/{m.ipc_total} | `apps/vesta-gui/browser-tests/vesta-mock.mjs` |",
        f"| IPC names referenced in Playwright specs | {m.ipc_spec_referenced}/{m.ipc_total} | `apps/vesta-gui/browser-tests/*.spec.mjs` |",
        f"| Vitest files / cases | {m.vitest_files} / {m.vitest_cases} | `apps/vesta-gui/src/**/*.test.*` |",
        f"| Playwright spec files / `test()` decls | {m.playwright_files} / {m.playwright_test_decls} | `apps/vesta-gui/browser-tests/*.spec.mjs` |",
        f"| MCP tools (`vesta-mcp`) | {m.mcp_tools} | `cli/vesta-mcp/src/lib.rs` |",
        f"| Unified `vesta-cli` workflows | {m.cli_workflows} | `cli/vesta-cli/src/lib.rs` |",
        f"| Plugin SDK widget primitives | {m.plugin_widgets} | `PluginRenderer.svelte` |",
        f"| Frontend localization catalogs | {m.locales} | `apps/vesta-gui/src/lib/i18n/locales/*.json` |",
        "",
        "## Executive Summary by Area",
        "",
        "| Area | Features | GUI | CLI | MCP | Docs | Unit Tests | Playwright | Smoke | Verified | Open Findings |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]

    def _cnt(items: list[dict[str, Any]], col: str, val: str = "yes") -> int:
        return sum(1 for x in items if x[col] == val)

    def _applicable_cnt(items: list[dict[str, Any]], col: str) -> int:
        return sum(1 for x in items if x[col] != "na")

    total_open_by_area: dict[str, int] = {a: 0 for a in AREA_LABELS}
    for entry in [*bugs, *perf]:
        if entry["status"] != "fixed":
            total_open_by_area[entry["area"]] += 1

    for area_key, area_title in AREA_LABELS.items():
        area_feats = [f for f in features if f["area"] == area_key]
        lines.append(
            f"| {area_title} | {len(area_feats)} "
            f"| {_cnt(area_feats, 'gui')}/{_applicable_cnt(area_feats, 'gui')} "
            f"| {_cnt(area_feats, 'cli')}/{_applicable_cnt(area_feats, 'cli')} "
            f"| {_cnt(area_feats, 'mcp')}/{_applicable_cnt(area_feats, 'mcp')} "
            f"| {_cnt(area_feats, 'docs')}/{len(area_feats)} "
            f"| {_cnt(area_feats, 'unit_tests')}/{len(area_feats)} "
            f"| {_cnt(area_feats, 'playwright')}/{_applicable_cnt(area_feats, 'playwright')} "
            f"| {_cnt(area_feats, 'smoke')}/{len(area_feats)} "
            f"| {_cnt(area_feats, 'audit_status', 'verified')}/{len(area_feats)} "
            f"| {total_open_by_area[area_key]} |"
        )

    lines.append(
        f"| **Total** | **{len(features)}** "
        f"| **{_cnt(features, 'gui')}/{_applicable_cnt(features, 'gui')}** "
        f"| **{_cnt(features, 'cli')}/{_applicable_cnt(features, 'cli')}** "
        f"| **{_cnt(features, 'mcp')}/{_applicable_cnt(features, 'mcp')}** "
        f"| **{_cnt(features, 'docs')}/{len(features)}** "
        f"| **{_cnt(features, 'unit_tests')}/{len(features)}** "
        f"| **{_cnt(features, 'playwright')}/{_applicable_cnt(features, 'playwright')}** "
        f"| **{_cnt(features, 'smoke')}/{len(features)}** "
        f"| **{_cnt(features, 'audit_status', 'verified')}/{len(features)}** "
        f"| **{sum(total_open_by_area.values())}** |"
    )

    # Audit findings status summary
    all_findings = [*bugs, *perf]
    fixed_cnt = sum(1 for x in all_findings if x["status"] == "fixed")
    open_cnt = len(all_findings) - fixed_cnt
    lines.extend(
        [
            "",
            "## Audit Findings Summary (106 Verified Stage-1 Findings)",
            "",
            f"- **Bugs & Hardening (`bugs.toml`):** {len(bugs)} total "
            f"({sum(1 for b in bugs if b['status'] == 'fixed')} fixed, "
            f"{sum(1 for b in bugs if b['status'] != 'fixed')} open/deferred)",
            f"- **Performance (`performance.toml`):** {len(perf)} total "
            f"({sum(1 for p in perf if p['status'] == 'fixed')} fixed, "
            f"{sum(1 for p in perf if p['status'] != 'fixed')} open)",
            f"- **Combined Progress:** {fixed_cnt}/{len(all_findings)} resolved, {open_cnt} remaining",
            "",
            "| Severity | Open / Deferred | Fixed | Total |",
            "|---|---:|---:|---:|",
        ]
    )
    for sev in ("critical", "high", "medium", "low"):
        s_items = [x for x in all_findings if x["severity"] == sev]
        s_fixed = sum(1 for x in s_items if x["status"] == "fixed")
        lines.append(f"| {sev.upper()} | {len(s_items) - s_fixed} | {s_fixed} | {len(s_items)} |")

    # Reliability controls
    lines.extend(
        [
            "",
            "## Reliability & Security Controls",
            "",
            "| ID | Area | Control | Status | Target | Notes |",
            "|---|---|---|---|---|---|",
        ]
    )
    for r in reliability:
        lines.append(
            f"| `{r['id']}` | {AREA_LABELS[r['area']]} | {_md_escape(r['control'])} "
            f"| {_badge(r['status'])} | `{_md_escape(r['target'])}` | {_md_escape(r['notes'])} |"
        )

    # Performance backlog
    lines.extend(
        [
            "",
            "## Performance Optimization Tracker",
            "",
            "| ID | Ref | Severity | Status | Target | Summary | Verification |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for p in perf:
        lines.append(
            f"| `{p['id']}` | `{p['audit_ref']}` | {p['severity'].upper()} | `{p['status']}` "
            f"| `{_md_escape(p['target'])}` | {_md_escape(p['summary'])} | {_md_escape(p['verification'])} |"
        )

    # Full Feature Coverage Matrix
    lines.extend(
        [
            "",
            "## Full Feature Coverage Matrix",
            "",
            "| ID | Area | Feature | GUI | CLI | MCP | Docs | Unit | Playwright | Smoke | Audit |",
            "|---|---|---|---|---|---|---|---|---|---|---|",
        ]
    )
    for f in features:
        lines.append(
            f"| `{f['id']}` | {AREA_LABELS[f['area']]} | {_md_escape(f['feature'])} "
            f"| {_badge(f['gui'])} | {_badge(f['cli'])} | {_badge(f['mcp'])} | {_badge(f['docs'])} "
            f"| {_badge(f['unit_tests'])} | {_badge(f['playwright'])} | {_badge(f['smoke'])} "
            f"| {_badge(f['audit_status'])} |"
        )

    # Fixed highlights
    fixed_items = [b for b in bugs if b["status"] == "fixed"]
    lines.extend(
        [
            "",
            "## Resolved Wave Findings",
            "",
        ]
    )
    if fixed_items:
        lines.extend(
            [
                "| ID | Ref | Severity | Area | Target | Resolution |",
                "|---|---|---|---|---|---|",
            ]
        )
        for b in fixed_items:
            lines.append(
                f"| `{b['id']}` | `{b['audit_ref']}` | {b['severity'].upper()} | {AREA_LABELS[b['area']]} "
                f"| `{_md_escape(b['target'])}` | {_md_escape(b['resolution'])} |"
            )
    else:
        lines.append("_No findings marked `fixed` yet in `scorecard/bugs.toml`._")

    # Open Critical & High backlog
    open_crit_high = [
        b for b in bugs if b["status"] != "fixed" and b["severity"] in {"critical", "high"}
    ]
    lines.extend(
        [
            "",
            "## Open Critical & High Findings",
            "",
            "Full 98-item bug/doc/test tracker lives in [`scorecard/bugs.toml`](../scorecard/bugs.toml).",
            "",
        ]
    )
    if open_crit_high:
        lines.extend(
            [
                "| ID | Ref | Severity | Area | Target | Summary |",
                "|---|---|---|---|---|---|",
            ]
        )
        for b in open_crit_high:
            lines.append(
                f"| `{b['id']}` | `{b['audit_ref']}` | {b['severity'].upper()} | {AREA_LABELS[b['area']]} "
                f"| `{_md_escape(b['target'])}` | {_md_escape(b['summary'])} |"
            )
    else:
        lines.append("_All critical and high findings resolved!_")

    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate scorecard TOML files and render docs/SCORECARD.md.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify scorecard/*.toml schema and fail if docs/SCORECARD.md is out of date.",
    )
    parser.add_argument(
        "--repo-root",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parents[1],
        help="Repository root directory.",
    )
    args = parser.parse_args(argv)
    repo_root = args.repo_root.resolve()

    parsed, errors = load_and_validate(repo_root)
    if errors:
        print("Scorecard validation failed:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    metrics = compute_metrics(repo_root)
    rendered = render_scorecard_md(parsed, metrics)
    out_path = repo_root / "docs/SCORECARD.md"

    if args.check:
        if not out_path.is_file():
            print(f"Missing {out_path}; run python3 build-scripts/generate_scorecard.py", file=sys.stderr)
            return 1
        current = out_path.read_text(encoding="utf-8")
        if current != rendered:
            print(
                f"{out_path.relative_to(repo_root)} is stale; run python3 build-scripts/generate_scorecard.py",
                file=sys.stderr,
            )
            return 1
        print(
            f"Scorecard OK ({len(parsed['features']['item'])} features, "
            f"{len(parsed['bugs']['item']) + len(parsed['performance']['item'])} tracked findings)."
        )
        return 0

    out_path.write_text(rendered, encoding="utf-8")
    print(f"Generated {out_path.relative_to(repo_root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

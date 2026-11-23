#!/usr/bin/env python3
"""Maintain, verify, and query the tracked SQLite database (`scorecard/scorecard.db`) for `scorecard/*.toml`."""

from __future__ import annotations

import argparse
from collections.abc import Iterator
import contextlib
import datetime
import fnmatch
import json
import os
import pathlib
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import tomllib
from typing import Any

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import generate_scorecard as gs  # noqa: E402

SCHEMA_USER_VERSION = 2

TOML_FILES = {
    "features": "features.toml",
    "reliability": "reliability.toml",
    "performance": "performance.toml",
    "bugs": "bugs.toml",
}
IMPACT_MAP_FILE = "impact_map.toml"

FEATURE_FIELDS = (
    "id",
    "area",
    "feature",
    "gui",
    "cli",
    "mcp",
    "docs",
    "unit_tests",
    "playwright",
    "smoke",
    "audit_status",
    "last_audited",
    "notes",
)
RELIABILITY_FIELDS = ("id", "area", "control", "status", "target", "evidence", "notes")
PERFORMANCE_FIELDS = (
    "id",
    "audit_ref",
    "area",
    "severity",
    "status",
    "target",
    "summary",
    "impact",
    "fix_plan",
    "verification",
)
BUG_FIELDS = (
    "id",
    "audit_ref",
    "area",
    "severity",
    "category",
    "status",
    "target",
    "summary",
    "resolution",
)

TABLE_FIELDS = {
    "features": FEATURE_FIELDS,
    "reliability": RELIABILITY_FIELDS,
    "performance": PERFORMANCE_FIELDS,
    "bugs": BUG_FIELDS,
}

SCHEMA_SQL = """
CREATE TABLE meta (
    source TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    last_updated TEXT NOT NULL,
    audit_wave TEXT NOT NULL
);

CREATE TABLE features (
    seq INTEGER PRIMARY KEY,
    id TEXT UNIQUE NOT NULL,
    area TEXT NOT NULL,
    feature TEXT NOT NULL,
    gui TEXT NOT NULL,
    cli TEXT NOT NULL,
    mcp TEXT NOT NULL,
    docs TEXT NOT NULL,
    unit_tests TEXT NOT NULL,
    playwright TEXT NOT NULL,
    smoke TEXT NOT NULL,
    audit_status TEXT NOT NULL,
    last_audited TEXT NOT NULL,
    notes TEXT NOT NULL
);

CREATE TABLE reliability (
    seq INTEGER PRIMARY KEY,
    id TEXT UNIQUE NOT NULL,
    area TEXT NOT NULL,
    control TEXT NOT NULL,
    status TEXT NOT NULL,
    target TEXT NOT NULL,
    evidence TEXT NOT NULL,
    notes TEXT NOT NULL
);

CREATE TABLE performance (
    seq INTEGER PRIMARY KEY,
    id TEXT UNIQUE NOT NULL,
    audit_ref TEXT UNIQUE NOT NULL,
    area TEXT NOT NULL,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    target TEXT NOT NULL,
    summary TEXT NOT NULL,
    impact TEXT NOT NULL,
    fix_plan TEXT NOT NULL,
    verification TEXT NOT NULL
);

CREATE TABLE bugs (
    seq INTEGER PRIMARY KEY,
    id TEXT UNIQUE NOT NULL,
    audit_ref TEXT UNIQUE NOT NULL,
    area TEXT NOT NULL,
    severity TEXT NOT NULL,
    category TEXT NOT NULL,
    status TEXT NOT NULL,
    target TEXT NOT NULL,
    summary TEXT NOT NULL,
    resolution TEXT NOT NULL
);

CREATE TABLE impact_rules (
    seq INTEGER PRIMARY KEY,
    id TEXT UNIQUE NOT NULL,
    area TEXT NOT NULL,
    feature_ids_json TEXT NOT NULL,
    paths_json TEXT NOT NULL,
    rust_crates_json TEXT NOT NULL,
    vitest_filters_json TEXT NOT NULL,
    playwright_specs_json TEXT NOT NULL
);

CREATE INDEX idx_features_area ON features(area);
CREATE INDEX idx_features_status ON features(audit_status);
CREATE INDEX idx_reliability_area ON reliability(area);
CREATE INDEX idx_reliability_status ON reliability(status);
CREATE INDEX idx_performance_area ON performance(area);
CREATE INDEX idx_performance_status ON performance(status);
CREATE INDEX idx_performance_severity ON performance(severity);
CREATE INDEX idx_bugs_area ON bugs(area);
CREATE INDEX idx_bugs_status ON bugs(status);
CREATE INDEX idx_bugs_severity ON bugs(severity);
"""


def _load_and_validate_impact_map(
    repo_root: pathlib.Path,
    parsed: dict[str, dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[str]]:
    """Load and validate `scorecard/impact_map.toml` against parsed feature IDs."""
    impact_file = repo_root / "scorecard" / IMPACT_MAP_FILE
    if not impact_file.is_file():
        return [], []

    errors: list[str] = []
    with impact_file.open("rb") as f:
        doc = tomllib.load(f)

    valid_feature_ids = {str(item["id"]) for item in parsed["features"]["item"]}
    rules = doc.get("rule", [])
    seen_ids: set[str] = set()
    validated: list[dict[str, Any]] = []

    for idx, rule in enumerate(rules):
        ctx = f"{IMPACT_MAP_FILE}[[{idx}]]"
        rule_id = str(rule.get("id", "")).strip()
        if not rule_id:
            errors.append(f"{ctx}: missing 'id'")
        elif rule_id in seen_ids:
            errors.append(f"{ctx}: duplicate rule id {rule_id!r}")
        seen_ids.add(rule_id)

        area = str(rule.get("area", "")).strip()
        if area not in gs.AREA_LABELS:
            errors.append(f"{ctx}: unknown area {area!r}")

        feature_ids = [str(v).strip() for v in rule.get("feature_ids", []) if str(v).strip()]
        if not feature_ids:
            errors.append(f"{ctx}: 'feature_ids' must be non-empty")
        for fid in feature_ids:
            if fid not in valid_feature_ids:
                errors.append(f"{ctx}: unknown feature_id {fid!r}")

        paths = [str(v).strip() for v in rule.get("paths", []) if str(v).strip()]
        if not paths:
            errors.append(f"{ctx}: 'paths' must be non-empty")

        validated.append(
            {
                "id": rule_id,
                "area": area,
                "feature_ids": feature_ids,
                "paths": paths,
                "rust_crates": [str(v).strip() for v in rule.get("rust_crates", []) if str(v).strip()],
                "vitest_filters": [str(v).strip() for v in rule.get("vitest_filters", []) if str(v).strip()],
                "playwright_specs": [str(v).strip() for v in rule.get("playwright_specs", []) if str(v).strip()],
            }
        )

    return validated, errors


def _db_path(repo_root: pathlib.Path) -> pathlib.Path:
    return repo_root / "scorecard" / "scorecard.db"


def _toml_str(value: str) -> str:
    """Format a Python string as a valid single-line TOML basic string."""
    out = ['"']
    for ch in value:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 0x20 or ord(ch) == 0x7F:
            out.append(f"\\u{ord(ch):04x}")
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def _serialize_toml(
    kind: str,
    meta: dict[str, Any],
    items: list[dict[str, Any]],
) -> str:
    fields = TABLE_FIELDS[kind]
    lines = [
        f"schema_version = {int(meta['schema_version'])}",
        f"last_updated = {_toml_str(str(meta['last_updated']))}",
    ]
    if meta.get("audit_wave"):
        lines.append(f"audit_wave = {_toml_str(str(meta['audit_wave']))}")
    if not items:
        lines.append("item = []")
    else:
        for item in items:
            lines.append("")
            lines.append("[[item]]")
            for field in fields:
                lines.append(f"{field} = {_toml_str(str(item[field]))}")
    lines.append("")
    return "\n".join(lines)


def _db_matches_parsed(
    db_file: pathlib.Path,
    parsed: dict[str, dict[str, Any]],
    impact_rules: list[dict[str, Any]] | None = None,
) -> tuple[bool, list[str]]:
    """Verify SQLite schema and logical table rows match parsed `scorecard/*.toml`."""
    mismatches: list[str] = []
    try:
        conn = sqlite3.connect(f"file:{db_file}?mode=ro", uri=True)
        try:
            ver = conn.execute("PRAGMA user_version").fetchone()[0]
            if ver != SCHEMA_USER_VERSION:
                mismatches.append(f"user_version={ver} (expected {SCHEMA_USER_VERSION})")
                return False, mismatches
            tables = {
                row[0]
                for row in conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='table'"
                ).fetchall()
            }
            required = {"meta", "impact_rules", *TOML_FILES.keys()}
            if not required <= tables:
                missing = sorted(required - tables)
                mismatches.append(f"missing tables: {missing}")
                return False, mismatches

            expected_meta = [
                (
                    source,
                    int(parsed[source]["schema_version"]),
                    str(parsed[source]["last_updated"]),
                    str(parsed[source].get("audit_wave", "")),
                )
                for source in sorted(TOML_FILES)
            ]
            actual_meta = conn.execute(
                "SELECT source, schema_version, last_updated, audit_wave FROM meta ORDER BY source"
            ).fetchall()
            if actual_meta != expected_meta:
                mismatches.append("meta table out of sync")

            for source in TOML_FILES:
                fields = TABLE_FIELDS[source]
                cols = ", ".join(["seq", *fields])
                expected_rows = [
                    tuple([seq, *(str(item[f]) for f in fields)])
                    for seq, item in enumerate(parsed[source]["item"], start=1)
                ]
                actual_rows = conn.execute(
                    f"SELECT {cols} FROM {source} ORDER BY seq"
                ).fetchall()
                if actual_rows != expected_rows:
                    mismatches.append(
                        f"{source} table out of sync ({len(actual_rows)} vs {len(expected_rows)} rows)"
                    )

            if impact_rules is not None:
                expected_imp = [
                    (
                        seq,
                        rule["id"],
                        rule["area"],
                        json.dumps(rule["feature_ids"]),
                        json.dumps(rule["paths"]),
                        json.dumps(rule["rust_crates"]),
                        json.dumps(rule["vitest_filters"]),
                        json.dumps(rule["playwright_specs"]),
                    )
                    for seq, rule in enumerate(impact_rules, start=1)
                ]
                actual_imp = conn.execute(
                    "SELECT seq, id, area, feature_ids_json, paths_json, rust_crates_json, vitest_filters_json, playwright_specs_json FROM impact_rules ORDER BY seq"
                ).fetchall()
                if actual_imp != expected_imp:
                    mismatches.append(
                        f"impact_rules table out of sync ({len(actual_imp)} vs {len(expected_imp)} rows)"
                    )
        finally:
            conn.close()
    except sqlite3.Error as exc:
        mismatches.append(f"sqlite error: {exc}")
        return False, mismatches

    return len(mismatches) == 0, mismatches


def _needs_rebuild(repo_root: pathlib.Path, db_file: pathlib.Path) -> bool:
    if not db_file.is_file():
        return True
    try:
        db_mtime = db_file.stat().st_mtime
        tracked_tomls = [repo_root / "scorecard" / fname for fname in TOML_FILES.values()]
        impact_file = repo_root / "scorecard" / IMPACT_MAP_FILE
        if impact_file.is_file():
            tracked_tomls.append(impact_file)
        newest_toml = max(p.stat().st_mtime for p in tracked_tomls)
    except OSError:
        return True
    try:
        conn = sqlite3.connect(f"file:{db_file}?mode=ro", uri=True)
        try:
            ver = conn.execute("PRAGMA user_version").fetchone()[0]
            if ver != SCHEMA_USER_VERSION:
                return True
            tables = {
                row[0]
                for row in conn.execute(
                    "SELECT name FROM sqlite_master WHERE type='table'"
                ).fetchall()
            }
            if not {"meta", "features", "reliability", "performance", "bugs", "impact_rules"} <= tables:
                return True
        finally:
            conn.close()
    except sqlite3.Error:
        return True
    if newest_toml > db_mtime:
        # On fresh clones / checkouts, TOML mtimes may exceed db_mtime even when
        # the tracked database already matches the TOMLs. Avoid dirtying git status.
        parsed, errors = gs.load_and_validate(repo_root)
        if errors:
            return True
        impact_rules, imp_errors = _load_and_validate_impact_map(repo_root, parsed)
        if imp_errors:
            return True
        matches, _ = _db_matches_parsed(db_file, parsed, impact_rules)
        return not matches
    return False


def _populate_db(
    conn: sqlite3.Connection,
    parsed: dict[str, dict[str, Any]],
    impact_rules: list[dict[str, Any]] | None = None,
) -> None:
    conn.executescript(SCHEMA_SQL)
    conn.execute(f"PRAGMA user_version = {SCHEMA_USER_VERSION}")
    for source in TOML_FILES:
        doc = parsed[source]
        conn.execute(
            "INSERT INTO meta (source, schema_version, last_updated, audit_wave) VALUES (?, ?, ?, ?)",
            (
                source,
                int(doc["schema_version"]),
                str(doc["last_updated"]),
                str(doc.get("audit_wave", "")),
            ),
        )
        fields = TABLE_FIELDS[source]
        cols = ", ".join(["seq", *fields])
        placeholders = ", ".join(["?"] * (1 + len(fields)))
        sql = f"INSERT INTO {source} ({cols}) VALUES ({placeholders})"
        rows = [
            tuple([seq, *(str(item[f]) for f in fields)])
            for seq, item in enumerate(doc["item"], start=1)
        ]
        conn.executemany(sql, rows)
    if impact_rules:
        conn.executemany(
            """
            INSERT INTO impact_rules (
                seq, id, area, feature_ids_json, paths_json,
                rust_crates_json, vitest_filters_json, playwright_specs_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            """,
            [
                (
                    seq,
                    rule["id"],
                    rule["area"],
                    json.dumps(rule["feature_ids"]),
                    json.dumps(rule["paths"]),
                    json.dumps(rule["rust_crates"]),
                    json.dumps(rule["vitest_filters"]),
                    json.dumps(rule["playwright_specs"]),
                )
                for seq, rule in enumerate(impact_rules, start=1)
            ],
        )


def ensure_db(repo_root: pathlib.Path, *, force: bool = False) -> pathlib.Path:
    """Ensure `scorecard/scorecard.db` exists and matches `scorecard/*.toml`."""
    repo_root = repo_root.resolve()
    db_file = _db_path(repo_root)
    if not force and not _needs_rebuild(repo_root, db_file):
        return db_file

    parsed, errors = gs.load_and_validate(repo_root)
    impact_rules, imp_errors = ([], []) if errors else _load_and_validate_impact_map(repo_root, parsed)
    all_errors = [*errors, *imp_errors]
    if all_errors:
        raise RuntimeError("Invalid scorecard TOML:\n  - " + "\n  - ".join(all_errors))

    db_dir = db_file.parent
    db_dir.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(dir=db_dir, prefix="scorecard-", suffix=".db.tmp")
    os.close(fd)
    tmp_path = pathlib.Path(tmp_name)
    try:
        conn = sqlite3.connect(tmp_path)
        try:
            _populate_db(conn, parsed, impact_rules)
            conn.commit()
        finally:
            conn.close()
        os.chmod(tmp_path, 0o644)
        os.replace(tmp_path, db_file)
    except Exception:
        tmp_path.unlink(missing_ok=True)
        raise
    return db_file


@contextlib.contextmanager
def _connect_ro(db_file: pathlib.Path) -> Iterator[sqlite3.Connection]:
    conn = sqlite3.connect(f"file:{db_file}?mode=ro", uri=True)
    try:
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA query_only = ON")
        yield conn
    finally:
        conn.close()


def _print_table(headers: list[str], rows: list[tuple[Any, ...]]) -> None:
    if not rows:
        print("(0 rows)")
        return
    str_rows = [[("" if v is None else str(v)) for v in row] for row in rows]
    widths = [
        max([len(h), *(len(r[i]) for r in str_rows)]) for i, h in enumerate(headers)
    ]
    fmt = "  ".join(f"{{:<{w}}}" for w in widths)
    print(fmt.format(*headers))
    print(fmt.format(*("-" * w for w in widths)))
    for row in str_rows:
        print(fmt.format(*row))


def _fetch_meta_summary(conn: sqlite3.Connection) -> sqlite3.Row | None:
    """Return canonical wave and latest update date across all meta rows."""
    return conn.execute(
        """
        SELECT
            COALESCE(
                (SELECT audit_wave FROM meta WHERE source = 'features' AND audit_wave != ''),
                (SELECT audit_wave FROM meta WHERE audit_wave != '' LIMIT 1),
                ''
            ) AS audit_wave,
            MAX(last_updated) AS last_updated
        FROM meta
        """
    ).fetchone()


def _cmd_sync(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root, force=True)
    with _connect_ro(db_file) as conn:
        counts = {
            table: conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            for table in ("features", "reliability", "performance", "bugs")
        }
        wave_row = _fetch_meta_summary(conn)
    payload = {
        "db_path": str(db_file),
        "audit_wave": wave_row["audit_wave"] if wave_row else "",
        "last_updated": wave_row["last_updated"] if wave_row else "",
        "counts": counts,
    }
    if args.json:
        print(json.dumps(payload, indent=2))
    else:
        print(
            f"Synced {db_file} (wave={payload['audit_wave']}, updated={payload['last_updated']}): "
            + ", ".join(f"{k}={v}" for k, v in counts.items())
        )
    return 0


def _cmd_check(args: argparse.Namespace) -> int:
    repo_root = args.repo_root.resolve()
    parsed, errors = gs.load_and_validate(repo_root)
    impact_rules, imp_errors = ([], []) if errors else _load_and_validate_impact_map(repo_root, parsed)
    all_errors = [*errors, *imp_errors]
    if all_errors:
        if args.json:
            print(json.dumps({"ok": False, "errors": all_errors}, indent=2))
        else:
            print("Invalid scorecard TOML:\n  - " + "\n  - ".join(all_errors), file=sys.stderr)
        return 1

    db_file = _db_path(repo_root)
    rel_db = db_file.relative_to(repo_root).as_posix()
    if not db_file.is_file():
        msg = f"{rel_db} is out of date; run 'python3 build-scripts/scorecard_db.py sync'"
        if args.json:
            print(json.dumps({"ok": False, "db_path": str(db_file), "errors": [msg]}, indent=2))
        else:
            print(msg, file=sys.stderr)
        return 1

    matches, mismatches = _db_matches_parsed(db_file, parsed, impact_rules)
    if not matches:
        detail = f" ({'; '.join(mismatches)})" if mismatches else ""
        msg = f"{rel_db} is out of date; run 'python3 build-scripts/scorecard_db.py sync'{detail}"
        if args.json:
            print(json.dumps({"ok": False, "db_path": str(db_file), "errors": [msg]}, indent=2))
        else:
            print(msg, file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps({"ok": True, "db_path": str(db_file)}, indent=2))
    else:
        print(f"{rel_db} is up to date.")
    return 0


def _cmd_summary(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    with _connect_ro(db_file) as conn:
        wave_row = _fetch_meta_summary(conn)
        bugs_by_status = dict(
            conn.execute("SELECT status, COUNT(*) FROM bugs GROUP BY status").fetchall()
        )
        bugs_open_sev = dict(
            conn.execute(
                "SELECT severity, COUNT(*) FROM bugs WHERE status != 'fixed' GROUP BY severity"
            ).fetchall()
        )
        perf_by_status = dict(
            conn.execute("SELECT status, COUNT(*) FROM performance GROUP BY status").fetchall()
        )
        rel_by_status = dict(
            conn.execute("SELECT status, COUNT(*) FROM reliability GROUP BY status").fetchall()
        )
        feat_by_status = dict(
            conn.execute("SELECT audit_status, COUNT(*) FROM features GROUP BY audit_status").fetchall()
        )
        areas = conn.execute(
            """
            SELECT
                f.area,
                COUNT(*) AS features_total,
                SUM(CASE WHEN f.audit_status = 'verified' THEN 1 ELSE 0 END) AS features_verified,
                SUM(CASE WHEN f.playwright IN ('partial', 'no') THEN 1 ELSE 0 END) AS playwright_gaps,
                SUM(CASE WHEN f.cli IN ('partial', 'no') THEN 1 ELSE 0 END) AS cli_gaps,
                SUM(CASE WHEN f.mcp IN ('partial', 'no') THEN 1 ELSE 0 END) AS mcp_gaps,
                COALESCE((SELECT COUNT(*) FROM bugs b WHERE b.area = f.area AND b.status != 'fixed'), 0) AS open_bugs,
                COALESCE((SELECT COUNT(*) FROM performance p WHERE p.area = f.area AND p.status != 'fixed'), 0) AS open_perf
            FROM features f
            GROUP BY f.area
            ORDER BY MIN(f.seq)
            """
        ).fetchall()

    area_rows = [dict(r) for r in areas]
    payload = {
        "audit_wave": wave_row["audit_wave"] if wave_row else "",
        "last_updated": wave_row["last_updated"] if wave_row else "",
        "bugs_by_status": bugs_by_status,
        "bugs_non_fixed_by_severity": bugs_open_sev,
        "performance_by_status": perf_by_status,
        "reliability_by_status": rel_by_status,
        "features_by_audit_status": feat_by_status,
        "areas": area_rows,
    }
    if args.json:
        print(json.dumps(payload, indent=2))
        return 0

    print(f"Scorecard ({payload['audit_wave']}, updated {payload['last_updated']})")
    print(f"  Bugs:        {bugs_by_status} (non-fixed severity: {bugs_open_sev or {}})")
    print(f"  Performance: {perf_by_status}")
    print(f"  Reliability: {rel_by_status}")
    print(f"  Features:    {feat_by_status}")
    print()
    _print_table(
        ["Area", "Feat", "Ver", "PW gap", "CLI gap", "MCP gap", "Open bugs", "Open perf"],
        [
            (
                r["area"],
                r["features_total"],
                r["features_verified"],
                r["playwright_gaps"],
                r["cli_gaps"],
                r["mcp_gaps"],
                r["open_bugs"],
                r["open_perf"],
            )
            for r in area_rows
        ],
    )
    return 0


def _cmd_open(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    clauses = ["status != 'fixed'"]
    params: list[Any] = []
    if not args.include_deferred:
        clauses.append("status != 'deferred'")
    if args.area:
        clauses.append("area = ?")
        params.append(args.area)
    if args.severity:
        clauses.append("severity = ?")
        params.append(args.severity)
    where = " AND ".join(clauses)
    sql = f"""
        SELECT * FROM (
            SELECT 'bug' AS kind, id, audit_ref, severity, status, area, target, summary
            FROM bugs WHERE {where}
            UNION ALL
            SELECT 'perf' AS kind, id, audit_ref, severity, status, area, target, summary
            FROM performance WHERE {where}
        )
        ORDER BY
            CASE severity WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END,
            kind, id
    """
    with _connect_ro(db_file) as conn:
        rows = [dict(r) for r in conn.execute(sql, params * 2).fetchall()]

    if args.json:
        print(json.dumps(rows, indent=2))
        return 0
    if not rows:
        print("No matching non-fixed findings.")
        return 0
    _print_table(
        ["Kind", "ID", "Ref", "Severity", "Status", "Area", "Target", "Summary"],
        [
            (
                r["kind"],
                r["id"],
                r["audit_ref"],
                r["severity"],
                r["status"],
                r["area"],
                r["target"],
                (r["summary"][:77] + "...") if len(r["summary"]) > 80 else r["summary"],
            )
            for r in rows
        ],
    )
    return 0


def _cmd_gaps(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    params: list[Any] = []
    clauses: list[str] = []
    if args.area:
        clauses.append("area = ?")
        params.append(args.area)

    if args.dim:
        dim = args.dim
        if dim == "audit_status":
            clauses.append("audit_status != 'verified'")
        else:
            clauses.append(f"{dim} IN ('partial', 'no')")
    else:
        clauses.append(
            "(audit_status != 'verified' OR gui IN ('partial', 'no') OR cli IN ('partial', 'no') "
            "OR mcp IN ('partial', 'no') OR docs IN ('partial', 'no') OR unit_tests IN ('partial', 'no') "
            "OR playwright IN ('partial', 'no') OR smoke IN ('partial', 'no'))"
        )

    where = "WHERE " + " AND ".join(clauses) if clauses else ""
    sql = f"""
        SELECT id, area, feature, gui, cli, mcp, docs, unit_tests, playwright, smoke, audit_status, notes
        FROM features
        {where}
        ORDER BY seq
    """
    with _connect_ro(db_file) as conn:
        rows = [dict(r) for r in conn.execute(sql, params).fetchall()]

    if args.json:
        print(json.dumps(rows, indent=2))
        return 0
    if not rows:
        print("No matching feature coverage gaps.")
        return 0

    dim_cols = ("gui", "cli", "mcp", "docs", "unit_tests", "playwright", "smoke")
    table_rows = []
    for r in rows:
        if args.dim:
            gap_desc = f"{args.dim}={r[args.dim]}"
        else:
            missing = [f"{c}={r[c]}" for c in dim_cols if r[c] in ("partial", "no")]
            gap_desc = ", ".join(missing) if missing else "-"
        feat_name = (r["feature"][:52] + "...") if len(r["feature"]) > 55 else r["feature"]
        table_rows.append((r["id"], r["area"], r["audit_status"], gap_desc, feat_name))
    _print_table(["ID", "Area", "Audit", "Gaps", "Feature"], table_rows)
    return 0


def _cmd_show(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    item_id = args.item_id
    with _connect_ro(db_file) as conn:
        for table in ("features", "bugs", "performance", "reliability"):
            row = conn.execute(
                f"SELECT * FROM {table} WHERE id = ? COLLATE NOCASE", (item_id,)
            ).fetchone()
            if row is not None:
                data = dict(row)
                data.pop("seq", None)
                if args.json:
                    print(json.dumps({"table": table, "item": data}, indent=2))
                else:
                    print(f"[{table}] {data['id']}")
                    for k, v in data.items():
                        if k != "id":
                            print(f"  {k}: {v}")
                return 0
    print(f"Item not found: {item_id}", file=sys.stderr)
    return 1


def _cmd_query(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    sql = args.sql.strip()
    if not sql:
        print("SQL query must not be empty.", file=sys.stderr)
        return 1
    try:
        with _connect_ro(db_file) as conn:
            cur = conn.execute(sql)
            headers = [desc[0] for desc in cur.description] if cur.description else []
            rows = cur.fetchall()
    except sqlite3.Error as exc:
        print(f"SQL query failed: {exc}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps([dict(r) for r in rows], indent=2))
        return 0
    if not headers:
        print("(ok)")
        return 0
    _print_table(headers, [tuple(r) for r in rows])
    return 0


def _path_matches_pattern(path_str: str, pattern: str) -> bool:
    clean = path_str.lstrip("./")
    if pattern.endswith("/**"):
        prefix = pattern[:-2]
        return clean.startswith(prefix)
    return fnmatch.fnmatchcase(clean, pattern)


def _collect_changed_files(repo_root: pathlib.Path, args: argparse.Namespace) -> list[str]:
    if args.files:
        return sorted({f.replace("\\", "/").lstrip("./") for f in args.files if f.strip()})

    changed: set[str] = set()
    if args.staged:
        proc = subprocess.run(
            ["git", "diff", "--cached", "--name-only"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False,
        )
        changed.update(line.strip() for line in proc.stdout.splitlines() if line.strip())
    else:
        proc = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False,
        )
        for line in proc.stdout.splitlines():
            if len(line) > 3:
                entry = line[3:].strip()
                if " -> " in entry:
                    entry = entry.split(" -> ", 1)[1].strip()
                changed.add(entry.strip('"'))
        if args.since:
            diff_proc = subprocess.run(
                ["git", "diff", "--name-only", f"{args.since}...HEAD"],
                cwd=repo_root,
                capture_output=True,
                text=True,
                check=False,
            )
            changed.update(line.strip() for line in diff_proc.stdout.splitlines() if line.strip())
    return sorted(changed)


def _cmd_impact(args: argparse.Namespace) -> int:
    repo_root = args.repo_root.resolve()
    db_file = ensure_db(repo_root)
    files = _collect_changed_files(repo_root, args)

    with _connect_ro(db_file) as conn:
        rule_rows = conn.execute("SELECT * FROM impact_rules ORDER BY seq").fetchall()
        features_by_id = {
            row["id"]: dict(row)
            for row in conn.execute("SELECT * FROM features ORDER BY seq").fetchall()
        }

    matched_rules: list[dict[str, Any]] = []
    impacted_feature_ids: set[str] = set()
    rust_crates: list[str] = []
    vitest_filters: list[str] = []
    playwright_specs: list[str] = []

    def _extend_unique(dest: list[str], items: list[str]) -> None:
        for item in items:
            if item not in dest:
                dest.append(item)

    for rule in rule_rows:
        patterns = json.loads(rule["paths_json"])
        rule_hits = [f for f in files if any(_path_matches_pattern(f, pat) for pat in patterns)]
        if not rule_hits:
            continue
        fids = json.loads(rule["feature_ids_json"])
        crates = json.loads(rule["rust_crates_json"])
        vfilters = json.loads(rule["vitest_filters_json"])
        pspecs = json.loads(rule["playwright_specs_json"])
        matched_rules.append({"id": rule["id"], "area": rule["area"], "matched_files": rule_hits, "feature_ids": fids})
        impacted_feature_ids.update(fids)
        _extend_unique(rust_crates, crates)
        _extend_unique(vitest_filters, vfilters)
        _extend_unique(playwright_specs, pspecs)

    impacted_features = sorted(
        (features_by_id[fid] for fid in impacted_feature_ids if fid in features_by_id),
        key=lambda f: f["seq"],
    )
    verified_features = [f for f in impacted_features if f["audit_status"] == "verified"]
    verified_gaps = [
        {
            "id": f["id"],
            "feature": f["feature"],
            "missing_surfaces": [
                dim for dim in ("unit_tests", "playwright", "smoke") if f[dim] in ("partial", "no")
            ],
        }
        for f in verified_features
        if any(f[dim] in ("partial", "no") for dim in ("unit_tests", "playwright", "smoke"))
    ]

    recommended_commands: list[list[str]] = []
    if rust_crates:
        cmd = ["cargo", "test", "--locked"]
        for crate in rust_crates:
            cmd.extend(["-p", crate])
        recommended_commands.append(cmd)
    if vitest_filters:
        recommended_commands.append(
            ["npm", "--prefix", "apps/vesta-gui", "test", "--", *vitest_filters]
        )
    if any(f.startswith("apps/vesta-gui/src/lib/i18n/") for f in files):
        recommended_commands.append(["bash", "build-scripts/check-i18n.sh"])
    if playwright_specs:
        recommended_commands.append(
            ["npm", "--prefix", "apps/vesta-gui", "run", "test:browser", "--", *playwright_specs]
        )

    payload = {
        "files": files,
        "matched_rules": matched_rules,
        "impacted_features": [
            {
                "id": f["id"],
                "area": f["area"],
                "audit_status": f["audit_status"],
                "unit_tests": f["unit_tests"],
                "playwright": f["playwright"],
                "smoke": f["smoke"],
                "feature": f["feature"],
            }
            for f in impacted_features
        ],
        "verified_count": len(verified_features),
        "verified_coverage_notes": verified_gaps,
        "rust_crates": rust_crates,
        "vitest_filters": vitest_filters,
        "playwright_specs": playwright_specs,
        "recommended_commands": [" ".join(cmd) for cmd in recommended_commands],
    }

    if args.strict_verified and verified_features and not (rust_crates or vitest_filters or playwright_specs):
        print("Strict verified check failed: no automated test suites mapped for verified features.", file=sys.stderr)
        return 1

    run_failures = 0
    if args.run and recommended_commands:
        for cmd in recommended_commands:
            if not args.playwright and "test:browser" in cmd:
                continue
            proc = subprocess.run(cmd, cwd=repo_root, check=False)
            if proc.returncode != 0:
                run_failures += 1

    if args.json:
        print(json.dumps(payload, indent=2))
        return 1 if run_failures else 0

    if not files:
        print("No changed files detected.")
        return 0

    print(f"Changed files analyzed: {len(files)}")
    if not impacted_features:
        print("No mapped scorecard features impacted.")
        return 0

    print(f"Impacted features: {len(impacted_features)} ({len(verified_features)} verified / solid)")
    _print_table(
        ["ID", "Area", "Audit", "Unit", "PW", "Smoke", "Feature"],
        [
            (
                f["id"],
                f["area"],
                f["audit_status"],
                f["unit_tests"],
                f["playwright"],
                f["smoke"],
                f["feature"],
            )
            for f in impacted_features
        ],
    )
    if verified_gaps:
        print("\nVerified features with non-complete test surfaces (extra care / manual review advised):")
        for g in verified_gaps:
            print(f"  - {g['id']}: {', '.join(g['missing_surfaces'])} -> {g['feature']}")
    if recommended_commands:
        print("\nRecommended re-verification commands:")
        for cmd in recommended_commands:
            print(f"  $ {' '.join(cmd)}")
    return 1 if run_failures else 0


def _apply_toml_mutation(
    repo_root: pathlib.Path,
    kind: str,
    mutator: Any,
    *,
    wave: str | None = None,
    render: bool = True,
) -> dict[str, Any]:
    """Validate and apply a mutation to `scorecard/<kind>.toml`, refresh SQLite DB and Markdown."""
    parsed, errors = gs.load_and_validate(repo_root)
    if errors:
        raise RuntimeError("Existing scorecard is invalid:\n  - " + "\n  - ".join(errors))

    doc = parsed[kind]
    today = datetime.date.today().isoformat()
    result_info = mutator(doc["item"], today)

    doc["last_updated"] = today
    if wave:
        doc["audit_wave"] = wave
        for other_kind, other_doc in parsed.items():
            if other_kind != kind:
                other_doc["audit_wave"] = wave

    new_text = _serialize_toml(kind, doc, doc["item"])

    # Stage full scorecard folder in a temp directory to validate before touching repo files.
    with tempfile.TemporaryDirectory(prefix="vesta-scorecard-stage-") as tmp:
        stage_root = pathlib.Path(tmp)
        shutil.copytree(repo_root / "scorecard", stage_root / "scorecard")
        # Exclude cache inside temp validation copy if present
        shutil.rmtree(stage_root / "scorecard" / ".cache", ignore_errors=True)
        for other_kind, other_doc in parsed.items():
            if other_kind == kind:
                (stage_root / "scorecard" / TOML_FILES[kind]).write_text(
                    new_text, encoding="utf-8"
                )
            elif wave:
                (stage_root / "scorecard" / TOML_FILES[other_kind]).write_text(
                    _serialize_toml(other_kind, other_doc, other_doc["item"]),
                    encoding="utf-8",
                )
        _, stage_errors = gs.load_and_validate(stage_root)
        if stage_errors:
            raise ValueError("Mutation rejected by scorecard validation:\n  - " + "\n  - ".join(stage_errors))

    scorecard_dir = repo_root / "scorecard"
    _atomic_write_text(scorecard_dir / TOML_FILES[kind], new_text)
    if wave:
        for other_kind, other_doc in parsed.items():
            if other_kind != kind:
                _atomic_write_text(
                    scorecard_dir / TOML_FILES[other_kind],
                    _serialize_toml(other_kind, other_doc, other_doc["item"]),
                )

    ensure_db(repo_root, force=True)
    if render:
        rc = gs.main(["--repo-root", str(repo_root)])
        if rc != 0:
            raise RuntimeError("generate_scorecard failed while rendering docs/SCORECARD.md")
    return result_info


def _atomic_write_text(path: pathlib.Path, content: str) -> None:
    fd, tmp_name = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}-", suffix=".tmp")
    os.close(fd)
    tmp_path = pathlib.Path(tmp_name)
    try:
        tmp_path.write_text(content, encoding="utf-8")
        os.replace(tmp_path, path)
    except Exception:
        tmp_path.unlink(missing_ok=True)
        raise


def _find_item(items: list[dict[str, Any]], item_id: str, label: str) -> dict[str, Any]:
    for item in items:
        if str(item["id"]).lower() == item_id.lower():
            return item
    raise KeyError(f"{label} ID not found: {item_id}")


def _cmd_update_bug(args: argparse.Namespace) -> int:
    updates = {
        k: v
        for k, v in {
            "status": args.status,
            "severity": args.severity,
            "category": args.category,
            "area": args.area,
            "target": args.target,
            "summary": args.summary,
            "resolution": args.resolution,
        }.items()
        if v is not None
    }
    if not updates:
        print("No fields specified to update.", file=sys.stderr)
        return 1

    def mutate(items: list[dict[str, Any]], _today: str) -> dict[str, Any]:
        target = _find_item(items, args.id, "Bug")
        target.update(updates)
        return dict(target)

    return _finish_mutation(args, "bugs", mutate)


def _cmd_update_perf(args: argparse.Namespace) -> int:
    updates = {
        k: v
        for k, v in {
            "status": args.status,
            "severity": args.severity,
            "area": args.area,
            "target": args.target,
            "summary": args.summary,
            "impact": args.impact,
            "fix_plan": args.fix_plan,
            "verification": args.verification,
        }.items()
        if v is not None
    }
    if not updates:
        print("No fields specified to update.", file=sys.stderr)
        return 1

    def mutate(items: list[dict[str, Any]], _today: str) -> dict[str, Any]:
        target = _find_item(items, args.id, "Performance")
        target.update(updates)
        return dict(target)

    return _finish_mutation(args, "performance", mutate)


def _cmd_update_feature(args: argparse.Namespace) -> int:
    updates = {
        k: v
        for k, v in {
            "feature": args.feature,
            "gui": args.gui,
            "cli": args.cli,
            "mcp": args.mcp,
            "docs": args.docs,
            "unit_tests": args.unit_tests,
            "playwright": args.playwright,
            "smoke": args.smoke,
            "audit_status": args.audit_status,
            "last_audited": args.last_audited,
            "notes": args.notes,
        }.items()
        if v is not None
    }
    if not updates:
        print("No fields specified to update.", file=sys.stderr)
        return 1

    def mutate(items: list[dict[str, Any]], today: str) -> dict[str, Any]:
        target = _find_item(items, args.id, "Feature")
        target.update(updates)
        if args.last_audited is None:
            target["last_audited"] = today
        return dict(target)

    return _finish_mutation(args, "features", mutate)


def _cmd_update_reliability(args: argparse.Namespace) -> int:
    updates = {
        k: v
        for k, v in {
            "area": args.area,
            "control": args.control,
            "status": args.status,
            "target": args.target,
            "evidence": args.evidence,
            "notes": args.notes,
        }.items()
        if v is not None
    }
    if not updates:
        print("No fields specified to update.", file=sys.stderr)
        return 1

    def mutate(items: list[dict[str, Any]], _today: str) -> dict[str, Any]:
        target = _find_item(items, args.id, "Reliability")
        target.update(updates)
        return dict(target)

    return _finish_mutation(args, "reliability", mutate)


def _next_audit_id(items: list[dict[str, Any]]) -> str:
    highest = 0
    for item in items:
        item_id = str(item.get("id", ""))
        if item_id.startswith("AUDIT-") and item_id[6:].isdigit():
            highest = max(highest, int(item_id[6:]))
    return f"AUDIT-{highest + 1:03d}"


def _cmd_add_bug(args: argparse.Namespace) -> int:
    def mutate(items: list[dict[str, Any]], _today: str) -> dict[str, Any]:
        new_id = args.id or _next_audit_id(items)
        entry = {
            "id": new_id,
            "audit_ref": args.audit_ref,
            "area": args.area,
            "severity": args.severity,
            "category": args.category,
            "status": args.status,
            "target": args.target,
            "summary": args.summary,
            "resolution": args.resolution,
        }
        items.append(entry)
        return dict(entry)

    return _finish_mutation(args, "bugs", mutate)


def _finish_mutation(args: argparse.Namespace, kind: str, mutator: Any) -> int:
    try:
        updated = _apply_toml_mutation(
            args.repo_root.resolve(),
            kind,
            mutator,
            wave=args.wave,
            render=not args.no_render,
        )
    except (KeyError, ValueError, RuntimeError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps(updated, indent=2))
    else:
        print(f"Updated {kind}:{updated['id']}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument(
        "--repo-root",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parents[1],
        help="Repository root directory.",
    )
    common.add_argument("--json", action="store_true", help="Emit structured JSON output.")

    mut_common = argparse.ArgumentParser(add_help=False, parents=[common])
    mut_common.add_argument("--wave", help="Optional audit_wave metadata string override.")
    mut_common.add_argument(
        "--no-render",
        action="store_true",
        help="Update TOML + SQLite DB without regenerating docs/SCORECARD.md.",
    )

    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="subcommand", required=True)

    p_sync = sub.add_parser("sync", parents=[common], help="Rebuild scorecard/scorecard.db from scorecard/*.toml.")
    p_sync.set_defaults(func=_cmd_sync)

    p_check = sub.add_parser("check", parents=[common], help="Verify scorecard/scorecard.db matches scorecard/*.toml.")
    p_check.set_defaults(func=_cmd_check)

    p_summary = sub.add_parser("summary", parents=[common], help="Print compact summary metrics and per-area gaps.")
    p_summary.set_defaults(func=_cmd_summary)

    p_open = sub.add_parser("open", parents=[common], help="List non-fixed bug and performance findings.")
    p_open.add_argument("--area", choices=sorted(gs.AREA_LABELS))
    p_open.add_argument("--severity", choices=sorted(gs.SEVERITY_VALUES))
    p_open.add_argument(
        "--include-deferred",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Include items marked status='deferred' (default: true).",
    )
    p_open.set_defaults(func=_cmd_open)

    p_gaps = sub.add_parser("gaps", parents=[common], help="List features with missing or partial coverage.")
    p_gaps.add_argument(
        "--dim",
        choices=("gui", "cli", "mcp", "docs", "unit_tests", "playwright", "smoke", "audit_status"),
        help="Filter gaps to a specific dimension (or omit for any partial/no/non-verified).",
    )
    p_gaps.add_argument("--area", choices=sorted(gs.AREA_LABELS))
    p_gaps.set_defaults(func=_cmd_gaps)

    p_show = sub.add_parser("show", parents=[common], help="Show full details for a single item ID.")
    p_show.add_argument("item_id")
    p_show.set_defaults(func=_cmd_show)

    p_query = sub.add_parser("query", parents=[common], help="Run a read-only SQL query against scorecard.db.")
    p_query.add_argument("sql")
    p_query.set_defaults(func=_cmd_query)

    p_impact = sub.add_parser(
        "impact",
        parents=[common],
        help="Map modified files to impacted scorecard features (especially verified ones) and list/run checks.",
    )
    p_impact.add_argument("--files", nargs="*", help="Explicit repo-relative paths instead of reading git status.")
    p_impact.add_argument("--since", help="Also include committed changes since this git ref (e.g. origin/dev).")
    p_impact.add_argument("--staged", action="store_true", help="Analyze staged git diff instead of working tree.")
    p_impact.add_argument("--run", action="store_true", help="Execute impacted Cargo & Vitest verification suites.")
    p_impact.add_argument("--playwright", action="store_true", help="With --run, also execute impacted Playwright browser specs.")
    p_impact.add_argument("--strict-verified", action="store_true", help="Fail if impacted verified features lack mapped test commands.")
    p_impact.set_defaults(func=_cmd_impact)

    p_ubug = sub.add_parser("update-bug", parents=[mut_common], help="Update a bug item in scorecard/bugs.toml.")
    p_ubug.add_argument("id")
    p_ubug.add_argument("--status", choices=sorted(gs.WORK_STATUS_VALUES))
    p_ubug.add_argument("--severity", choices=sorted(gs.SEVERITY_VALUES))
    p_ubug.add_argument("--category", choices=sorted(gs.BUG_CATEGORY_VALUES))
    p_ubug.add_argument("--area", choices=sorted(gs.AREA_LABELS))
    p_ubug.add_argument("--target")
    p_ubug.add_argument("--summary")
    p_ubug.add_argument("--resolution")
    p_ubug.set_defaults(func=_cmd_update_bug)

    p_uperf = sub.add_parser("update-perf", parents=[mut_common], help="Update a performance item in scorecard/performance.toml.")
    p_uperf.add_argument("id")
    p_uperf.add_argument("--status", choices=sorted(gs.WORK_STATUS_VALUES))
    p_uperf.add_argument("--severity", choices=sorted(gs.SEVERITY_VALUES))
    p_uperf.add_argument("--area", choices=sorted(gs.AREA_LABELS))
    p_uperf.add_argument("--target")
    p_uperf.add_argument("--summary")
    p_uperf.add_argument("--impact")
    p_uperf.add_argument("--fix-plan")
    p_uperf.add_argument("--verification")
    p_uperf.set_defaults(func=_cmd_update_perf)

    p_ufeat = sub.add_parser("update-feature", parents=[mut_common], help="Update a feature item in scorecard/features.toml.")
    p_ufeat.add_argument("id")
    p_ufeat.add_argument("--feature")
    p_ufeat.add_argument("--gui", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--cli", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--mcp", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--docs", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--unit-tests", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--playwright", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--smoke", choices=sorted(gs.COVERAGE_VALUES))
    p_ufeat.add_argument("--audit-status", choices=sorted(gs.AUDIT_STATUS_VALUES))
    p_ufeat.add_argument("--last-audited")
    p_ufeat.add_argument("--notes")
    p_ufeat.set_defaults(func=_cmd_update_feature)

    p_urel = sub.add_parser("update-reliability", parents=[mut_common], help="Update a reliability control in scorecard/reliability.toml.")
    p_urel.add_argument("id")
    p_urel.add_argument("--area", choices=sorted(gs.AREA_LABELS))
    p_urel.add_argument("--control")
    p_urel.add_argument("--status", choices=sorted(gs.RELIABILITY_STATUS_VALUES))
    p_urel.add_argument("--target")
    p_urel.add_argument("--evidence")
    p_urel.add_argument("--notes")
    p_urel.set_defaults(func=_cmd_update_reliability)

    p_add_bug = sub.add_parser("add-bug", parents=[mut_common], help="Add a new bug finding to scorecard/bugs.toml.")
    p_add_bug.add_argument("--id", help="Bug ID (auto-assigned AUDIT-NNN if omitted).")
    p_add_bug.add_argument("--audit-ref", required=True)
    p_add_bug.add_argument("--area", required=True, choices=sorted(gs.AREA_LABELS))
    p_add_bug.add_argument("--severity", required=True, choices=sorted(gs.SEVERITY_VALUES))
    p_add_bug.add_argument("--category", required=True, choices=sorted(gs.BUG_CATEGORY_VALUES))
    p_add_bug.add_argument("--status", default="open", choices=sorted(gs.WORK_STATUS_VALUES))
    p_add_bug.add_argument("--target", required=True)
    p_add_bug.add_argument("--summary", required=True)
    p_add_bug.add_argument("--resolution", default="")
    p_add_bug.set_defaults(func=_cmd_add_bug)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())

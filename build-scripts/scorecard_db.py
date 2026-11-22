#!/usr/bin/env python3
"""Maintain and query a local SQLite cache (`scorecard/.cache/scorecard.db`) for `scorecard/*.toml`."""

from __future__ import annotations

import argparse
import datetime
import json
import os
import pathlib
import shutil
import sqlite3
import sys
import tempfile
from typing import Any

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

import generate_scorecard as gs  # noqa: E402

SCHEMA_USER_VERSION = 1

TOML_FILES = {
    "features": "features.toml",
    "reliability": "reliability.toml",
    "performance": "performance.toml",
    "bugs": "bugs.toml",
}

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


def _db_path(repo_root: pathlib.Path) -> pathlib.Path:
    return repo_root / "scorecard" / ".cache" / "scorecard.db"


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
        f"audit_wave = {_toml_str(str(meta['audit_wave']))}",
    ]
    for item in items:
        lines.append("")
        lines.append("[[item]]")
        for field in fields:
            lines.append(f"{field} = {_toml_str(str(item[field]))}")
    lines.append("")
    return "\n".join(lines)


def _needs_rebuild(repo_root: pathlib.Path, db_file: pathlib.Path) -> bool:
    if not db_file.is_file():
        return True
    try:
        db_mtime = db_file.stat().st_mtime
        newest_toml = max((repo_root / "scorecard" / fname).stat().st_mtime for fname in TOML_FILES.values())
    except OSError:
        return True
    if newest_toml > db_mtime:
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
            if not {"meta", "features", "reliability", "performance", "bugs"} <= tables:
                return True
        finally:
            conn.close()
    except sqlite3.Error:
        return True
    return False


def _populate_db(conn: sqlite3.Connection, parsed: dict[str, dict[str, Any]]) -> None:
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


def ensure_db(repo_root: pathlib.Path, *, force: bool = False) -> pathlib.Path:
    """Ensure `scorecard/.cache/scorecard.db` exists and is fresher than `scorecard/*.toml`."""
    repo_root = repo_root.resolve()
    db_file = _db_path(repo_root)
    if not force and not _needs_rebuild(repo_root, db_file):
        return db_file

    parsed, errors = gs.load_and_validate(repo_root)
    if errors:
        raise RuntimeError("Invalid scorecard TOML:\n  - " + "\n  - ".join(errors))

    cache_dir = db_file.parent
    cache_dir.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(dir=cache_dir, prefix="scorecard-", suffix=".db.tmp")
    os.close(fd)
    tmp_path = pathlib.Path(tmp_name)
    try:
        conn = sqlite3.connect(tmp_path)
        try:
            _populate_db(conn, parsed)
            conn.commit()
        finally:
            conn.close()
        os.replace(tmp_path, db_file)
    except Exception:
        tmp_path.unlink(missing_ok=True)
        raise
    return db_file


def _connect_ro(db_file: pathlib.Path) -> sqlite3.Connection:
    conn = sqlite3.connect(f"file:{db_file}?mode=ro", uri=True)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA query_only = ON")
    return conn


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


def _cmd_sync(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root, force=True)
    with _connect_ro(db_file) as conn:
        counts = {
            table: conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
            for table in ("features", "reliability", "performance", "bugs")
        }
        wave_row = conn.execute(
            "SELECT audit_wave, MAX(last_updated) AS last_updated FROM meta"
        ).fetchone()
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


def _cmd_summary(args: argparse.Namespace) -> int:
    db_file = ensure_db(args.repo_root)
    with _connect_ro(db_file) as conn:
        wave_row = conn.execute(
            "SELECT audit_wave, MAX(last_updated) AS last_updated FROM meta"
        ).fetchone()
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

    p_sync = sub.add_parser("sync", parents=[common], help="Rebuild SQLite cache from scorecard/*.toml.")
    p_sync.set_defaults(func=_cmd_sync)

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

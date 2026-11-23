#!/usr/bin/env python3
"""Unit tests for build-scripts/generate_scorecard.py, scorecard_db.py, and repository scorecard TOMLs."""

from __future__ import annotations

import contextlib
import io
import json
import os
import pathlib
import shutil
import tempfile
import unittest

import generate_scorecard as gs
from generate_scorecard import compute_metrics, load_and_validate, main, render_scorecard_md
import scorecard_db as sdb

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def _write_sample_bugs(
    scorecard_dir: pathlib.Path,
    entries: list[dict[str, str]],
) -> None:
    lines = [
        "schema_version = 1",
        'last_updated = "2026-11-23"',
    ]
    if not entries:
        lines.append("item = []")
    else:
        for entry in entries:
            lines.extend(["", "[[item]]"])
            for field in sdb.BUG_FIELDS:
                lines.append(f"{field} = {sdb._toml_str(entry[field])}")
    lines.append("")
    (scorecard_dir / "bugs.toml").write_text("\n".join(lines), encoding="utf-8")


def _write_sample_perf(
    scorecard_dir: pathlib.Path,
    entries: list[dict[str, str]],
) -> None:
    lines = [
        "schema_version = 1",
        'last_updated = "2026-11-23"',
    ]
    if not entries:
        lines.append("item = []")
    else:
        for entry in entries:
            lines.extend(["", "[[item]]"])
            for field in sdb.PERFORMANCE_FIELDS:
                lines.append(f"{field} = {sdb._toml_str(entry[field])}")
    lines.append("")
    (scorecard_dir / "performance.toml").write_text("\n".join(lines), encoding="utf-8")


class ScorecardTests(unittest.TestCase):
    def test_repo_scorecard_tomls_are_valid_and_complete(self) -> None:
        parsed, errors = load_and_validate(REPO_ROOT)
        self.assertEqual(errors, [])
        self.assertEqual(len(parsed["features"]["item"]), 50)
        self.assertGreaterEqual(len(parsed["reliability"]["item"]), 10)
        self.assertEqual(len(parsed["performance"]["item"]), 0)
        self.assertEqual(len(parsed["bugs"]["item"]), 0)

    def test_empty_features_or_reliability_is_rejected(self) -> None:
        for fname in ("features.toml", "reliability.toml"):
            with tempfile.TemporaryDirectory() as tmp:
                root = pathlib.Path(tmp)
                shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
                (root / "scorecard" / fname).write_text(
                    'schema_version = 1\nlast_updated = "2026-11-23"\nitem = []\n',
                    encoding="utf-8",
                )
                _, errors = load_and_validate(root)
                self.assertTrue(
                    any(f"{fname}: must define non-empty [[item]] entries" in e for e in errors),
                    errors,
                )

    def test_duplicate_audit_ref_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            _write_sample_bugs(
                root / "scorecard",
                [
                    {
                        "id": "AUDIT-001",
                        "audit_ref": "#8.1",
                        "area": "plugins",
                        "severity": "high",
                        "category": "security",
                        "status": "open",
                        "target": "apps/vesta-gui/src-tauri/src/plugins/mod.rs:1",
                        "summary": "First sample bug",
                        "resolution": "",
                    },
                    {
                        "id": "AUDIT-002",
                        "audit_ref": "#8.1",
                        "area": "media_security",
                        "severity": "high",
                        "category": "security",
                        "status": "open",
                        "target": "apps/vesta-gui/src-tauri/src/media_stream.rs:1",
                        "summary": "Duplicate audit_ref sample bug",
                        "resolution": "",
                    },
                ],
            )
            _, errors = load_and_validate(root)
            self.assertTrue(any("duplicate audit_ref '#8.1'" in e for e in errors), errors)

    def test_fixed_bug_requires_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            _write_sample_bugs(
                root / "scorecard",
                [
                    {
                        "id": "AUDIT-001",
                        "audit_ref": "#8.1",
                        "area": "plugins",
                        "severity": "high",
                        "category": "bug",
                        "status": "fixed",
                        "target": "apps/vesta-gui/src-tauri/src/plugins/mod.rs:1",
                        "summary": "Fixed item missing resolution",
                        "resolution": "",
                    }
                ],
            )
            _, errors = load_and_validate(root)
            self.assertTrue(any("requires non-empty 'resolution'" in e for e in errors), errors)

    def test_metrics_scan_and_render_smoke(self) -> None:
        parsed, errors = load_and_validate(REPO_ROOT)
        self.assertEqual(errors, [])
        metrics = compute_metrics(REPO_ROOT)
        self.assertEqual(metrics.workspace_crates, 24)
        self.assertEqual(metrics.ipc_total, 109)
        self.assertEqual(metrics.mcp_tools, 24)
        self.assertEqual(metrics.plugin_widgets, 23)
        self.assertEqual(metrics.locales, 15)
        md = render_scorecard_md(parsed, metrics)
        self.assertIn("# Vesta Feature Scorecard & Audit Dashboard", md)
        self.assertNotIn("AUDIT_LOG.md", md)
        self.assertNotIn("106 Verified Stage-1 Findings", md)
        self.assertNotIn("98-item", md)
        self.assertIn("excluding `N/A`", md)
        self.assertIn("_No open bugs._", md)
        self.assertIn("_No open performance optimizations queued._", md)

    def test_render_active_findings_fixture(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            _write_sample_bugs(
                root / "scorecard",
                [
                    {
                        "id": "AUDIT-001",
                        "audit_ref": "#99.1",
                        "area": "cli_mcp",
                        "severity": "high",
                        "category": "bug",
                        "status": "open",
                        "target": "cli/vesta-cli/src/lib.rs:1",
                        "summary": "Active bug fixture for markdown rendering",
                        "resolution": "",
                    }
                ],
            )
            _write_sample_perf(
                root / "scorecard",
                [
                    {
                        "id": "PERF-001",
                        "audit_ref": "#99.2",
                        "area": "extract",
                        "severity": "medium",
                        "status": "in_progress",
                        "target": "lib/vesta-ocr/src/bitmap.rs:1",
                        "summary": "Active performance fixture for markdown rendering",
                        "impact": "Reduces cue decoding latency",
                        "fix_plan": "Batch ffmpeg probe calls",
                        "verification": "Pending benchmark",
                    }
                ],
            )
            parsed, errors = load_and_validate(root)
            self.assertEqual(errors, [])
            metrics = compute_metrics(REPO_ROOT)
            md = render_scorecard_md(parsed, metrics)
            self.assertIn("## Open Findings", md)
            self.assertIn("## Performance Tracker", md)
            self.assertIn("`AUDIT-001`", md)
            self.assertIn("`PERF-001`", md)
            self.assertNotIn("_No open bugs._", md)
            self.assertNotIn("_No open performance optimizations queued._", md)

    def test_check_detects_stale_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            (root / "docs").mkdir(parents=True)
            (root / "docs/SCORECARD.md").write_text("# stale\n", encoding="utf-8")
            rc = main(["--check", "--repo-root", str(root)])
            self.assertEqual(rc, 1)

    def test_sqlite_sync_and_queries(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            (root / "scorecard" / "scorecard.db").unlink(missing_ok=True)
            shutil.rmtree(root / "scorecard" / ".cache", ignore_errors=True)

            def run_cli(*argv: str) -> tuple[int, str, str]:
                out_buf, err_buf = io.StringIO(), io.StringIO()
                with contextlib.redirect_stdout(out_buf), contextlib.redirect_stderr(err_buf):
                    code = sdb.main([*argv, "--repo-root", str(root)])
                return code, out_buf.getvalue(), err_buf.getvalue()

            rc, out, _ = run_cli("sync", "--json")
            self.assertEqual(rc, 0)
            sync_payload = json.loads(out)
            self.assertEqual(sync_payload["counts"]["features"], 50)
            self.assertEqual(sync_payload["counts"]["bugs"], 0)
            self.assertEqual(sync_payload["counts"]["performance"], 0)
            self.assertTrue(sync_payload["audit_wave"])

            rc, _, _ = run_cli("check", "--json")
            self.assertEqual(rc, 0)

            rc, out, _ = run_cli("summary", "--json")
            self.assertEqual(rc, 0)
            summary = json.loads(out)
            self.assertEqual(summary["bugs_by_status"], {})
            self.assertEqual(summary["performance_by_status"], {})

            rc, out, _ = run_cli("open", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [])

            rc, out, _ = run_cli("gaps", "--dim", "playwright", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [])

            rc, out, _ = run_cli("gaps", "--dim", "mcp", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [])

            rc, out, _ = run_cli("gaps", "--dim", "smoke", "--json")
            self.assertEqual(rc, 0)
            smoke_gaps = json.loads(out)
            self.assertGreater(len(smoke_gaps), 0)
            self.assertTrue(all(item["smoke"] in ("partial", "no") for item in smoke_gaps))

            rc, out, _ = run_cli("show", "FC-01", "--json")
            self.assertEqual(rc, 0)
            shown = json.loads(out)
            self.assertEqual(shown["table"], "features")
            self.assertEqual(shown["item"]["id"], "FC-01")

            rc, _, _ = run_cli("show", "AUDIT-001", "--json")
            self.assertNotEqual(rc, 0)

            rc, out, _ = run_cli("query", "SELECT id FROM bugs WHERE status != 'fixed'", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [])

            rc, _, err = run_cli("query", "DELETE FROM bugs")
            self.assertNotEqual(rc, 0)
            self.assertTrue(err.strip())
            rc, out, _ = run_cli("query", "SELECT COUNT(*) AS cnt FROM bugs", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [{"cnt": 0}])

            # Touching TOML mtimes after checkout without content changes must not rewrite scorecard.db.
            db_path = root / "scorecard" / "scorecard.db"
            before_stat = db_path.stat()
            features_toml = root / "scorecard" / "features.toml"
            os.utime(features_toml, (before_stat.st_atime, before_stat.st_mtime + 10.0))
            rc, _, _ = run_cli("summary", "--json")
            self.assertEqual(rc, 0)
            after_stat = db_path.stat()
            self.assertEqual(after_stat.st_ino, before_stat.st_ino)
            self.assertEqual(after_stat.st_mtime_ns, before_stat.st_mtime_ns)

            # Mutating TOML without syncing must fail `check`.
            text = features_toml.read_text(encoding="utf-8")
            features_toml.write_text(text.replace('notes = "', 'notes = "Changed: ', 1), encoding="utf-8")
            rc, _, _ = run_cli("check", "--json")
            self.assertEqual(rc, 1)

    def test_sqlite_mutations_roundtrip_toml_and_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            (root / "scorecard" / "scorecard.db").unlink(missing_ok=True)
            shutil.rmtree(root / "scorecard" / ".cache", ignore_errors=True)
            (root / "docs").mkdir(parents=True)

            # Adding first bug to empty bugs.toml must auto-assign AUDIT-001.
            rc = sdb.main(
                [
                    "add-bug",
                    "--audit-ref",
                    "#99.1",
                    "--area",
                    "cli_mcp",
                    "--severity",
                    "high",
                    "--category",
                    "bug",
                    "--target",
                    "cli/vesta-cli/src/lib.rs:1",
                    "--summary",
                    "Initial synthetic open bug",
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)
            md_after_first = (root / "docs/SCORECARD.md").read_text(encoding="utf-8")
            self.assertIn("`AUDIT-001`", md_after_first)
            self.assertNotIn("_No open bugs._", md_after_first)

            rc = sdb.main(
                [
                    "update-bug",
                    "AUDIT-001",
                    "--status",
                    "fixed",
                    "--resolution",
                    'Verified CLI & MCP parity with quotes "ok" and slash \\ line1',
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)
            md_after_fix = (root / "docs/SCORECARD.md").read_text(encoding="utf-8")
            self.assertIn("_No open bugs._", md_after_fix)

            rc = sdb.main(
                [
                    "update-feature",
                    "FC-02",
                    "--playwright",
                    "yes",
                    "--notes",
                    "Added full series Playwright coverage",
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)

            rc = sdb.main(
                [
                    "add-bug",
                    "--audit-ref",
                    "#99.2",
                    "--area",
                    "cli_mcp",
                    "--severity",
                    "low",
                    "--category",
                    "test_gap",
                    "--target",
                    "cli/vesta-cli/src/lib.rs:2",
                    "--summary",
                    "Synthetic follow-up test item",
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)

            # Also exercise update-perf roundtrip on a sample performance item.
            _write_sample_perf(
                root / "scorecard",
                [
                    {
                        "id": "PERF-001",
                        "audit_ref": "#99.3",
                        "area": "extract",
                        "severity": "medium",
                        "status": "open",
                        "target": "lib/vesta-ocr/src/bitmap.rs:1",
                        "summary": "Sample performance item",
                        "impact": "Faster OCR",
                        "fix_plan": "Batch frames",
                        "verification": "Pending",
                    }
                ],
            )
            rc = sdb.main(
                [
                    "update-perf",
                    "PERF-001",
                    "--status",
                    "fixed",
                    "--verification",
                    "Verified via unit benchmark",
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)

            parsed, errors = load_and_validate(root)
            self.assertEqual(errors, [])
            self.assertEqual(len(parsed["bugs"]["item"]), 2)
            audit_01 = next(b for b in parsed["bugs"]["item"] if b["id"] == "AUDIT-001")
            self.assertEqual(audit_01["status"], "fixed")
            self.assertIn('"ok"', audit_01["resolution"])
            fc_02 = next(f for f in parsed["features"]["item"] if f["id"] == "FC-02")
            self.assertEqual(fc_02["playwright"], "yes")

            out_buf = io.StringIO()
            with contextlib.redirect_stdout(out_buf):
                rc = sdb.main(
                    ["open", "--json", "--repo-root", str(root)]
                )
            self.assertEqual(rc, 0)
            open_rows = json.loads(out_buf.getvalue())
            self.assertEqual([r["id"] for r in open_rows], ["AUDIT-002"])

            out_buf = io.StringIO()
            with contextlib.redirect_stdout(out_buf):
                rc = sdb.main(
                    ["show", "AUDIT-001", "--json", "--repo-root", str(root)]
                )
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out_buf.getvalue())["item"]["status"], "fixed")

            out_buf = io.StringIO()
            with contextlib.redirect_stdout(out_buf):
                rc = sdb.main(
                    ["query", "SELECT id, status FROM bugs WHERE id = 'AUDIT-002'", "--json", "--repo-root", str(root)]
                )
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out_buf.getvalue()), [{"id": "AUDIT-002", "status": "open"}])

            self.assertEqual(gs.main(["--check", "--repo-root", str(root)]), 0)

    def test_impact_mapping_and_invalid_feature_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            (root / "scorecard" / "scorecard.db").unlink(missing_ok=True)
            shutil.rmtree(root / "scorecard" / ".cache", ignore_errors=True)

            out_buf = io.StringIO()
            with contextlib.redirect_stdout(out_buf):
                rc = sdb.main([
                    "impact",
                    "--files",
                    "lib/vesta-sync/src/matching.rs",
                    "apps/vesta-gui/src/lib/tabs/AlignTab.svelte",
                    "--strict-verified",
                    "--json",
                    "--repo-root",
                    str(root),
                ])
            self.assertEqual(rc, 0)
            payload = json.loads(out_buf.getvalue())
            impacted_ids = {f["id"] for f in payload["impacted_features"]}
            self.assertTrue({"SY-01", "SY-02", "SY-03"} <= impacted_ids)
            self.assertGreaterEqual(payload["verified_count"], 1)
            self.assertIn("vesta-sync", payload["rust_crates"])
            self.assertIn("workflows.spec.mjs", payload["playwright_specs"])

            # Invalid feature ID in impact_map.toml must fail check.
            impact_toml = root / "scorecard" / "impact_map.toml"
            text = impact_toml.read_text(encoding="utf-8")
            impact_toml.write_text(text.replace('"SY-03"', '"NOPE-99"', 1), encoding="utf-8")
            err_buf = io.StringIO()
            with contextlib.redirect_stderr(err_buf):
                rc = sdb.main(["check", "--repo-root", str(root)])
            self.assertEqual(rc, 1)
            self.assertIn("NOPE-99", err_buf.getvalue())


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Unit tests for build-scripts/generate_scorecard.py and repository scorecard TOMLs."""

from __future__ import annotations

import contextlib
import io
import json
import os
import pathlib
import re
import shutil
import tempfile
import unittest

import generate_scorecard as gs
from generate_scorecard import compute_metrics, load_and_validate, main, render_scorecard_md
import scorecard_db as sdb

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


class ScorecardTests(unittest.TestCase):
    def test_repo_scorecard_tomls_are_valid_and_complete(self) -> None:
        parsed, errors = load_and_validate(REPO_ROOT)
        self.assertEqual(errors, [])
        self.assertEqual(len(parsed["features"]["item"]), 50)
        self.assertEqual(len(parsed["performance"]["item"]), 8)
        self.assertEqual(len(parsed["bugs"]["item"]), 98)

    def test_duplicate_audit_ref_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            bugs_path = root / "scorecard/bugs.toml"
            text = bugs_path.read_text(encoding="utf-8")
            text = text.replace('audit_ref = "#8.2"', 'audit_ref = "#8.1"', 1)
            bugs_path.write_text(text, encoding="utf-8")
            _, errors = load_and_validate(root)
            self.assertTrue(any("duplicate audit_ref '#8.1'" in e for e in errors), errors)

    def test_fixed_bug_requires_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            bugs_path = root / "scorecard/bugs.toml"
            text = bugs_path.read_text(encoding="utf-8")
            text = re.sub(r'resolution = "[^"]+"', 'resolution = ""', text, count=1)
            bugs_path.write_text(text, encoding="utf-8")
            _, errors = load_and_validate(root)
            self.assertTrue(any("requires non-empty 'resolution'" in e for e in errors), errors)

    def test_metrics_scan_and_render_smoke(self) -> None:
        parsed, errors = load_and_validate(REPO_ROOT)
        self.assertEqual(errors, [])
        metrics = compute_metrics(REPO_ROOT)
        self.assertEqual(metrics.workspace_crates, 24)
        self.assertEqual(metrics.ipc_total, 109)
        self.assertEqual(metrics.mcp_tools, 18)
        self.assertEqual(metrics.plugin_widgets, 23)
        self.assertEqual(metrics.locales, 15)
        md = render_scorecard_md(parsed, metrics)
        self.assertIn("# Vesta Feature Scorecard & Audit Dashboard", md)
        self.assertIn("`AUDIT-001`", md)
        self.assertIn("`PERF-001`", md)

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
            self.assertEqual(sync_payload["counts"]["bugs"], 98)

            rc, _, _ = run_cli("check", "--json")
            self.assertEqual(rc, 0)

            rc, out, _ = run_cli("summary", "--json")
            self.assertEqual(rc, 0)
            summary = json.loads(out)
            self.assertIsNone(summary["bugs_by_status"].get("deferred"))
            self.assertEqual(summary["bugs_by_status"].get("fixed"), 98)

            rc, out, _ = run_cli("open", "--json")
            self.assertEqual(rc, 0)
            open_items = json.loads(out)
            self.assertEqual(open_items, [])

            rc, out, _ = run_cli("gaps", "--dim", "playwright", "--json")
            self.assertEqual(rc, 0)
            pw_gaps = json.loads(out)
            self.assertEqual(pw_gaps, [])

            rc, out, _ = run_cli("gaps", "--dim", "mcp", "--json")
            self.assertEqual(rc, 0)
            mcp_gaps = json.loads(out)
            self.assertGreater(len(mcp_gaps), 0)
            self.assertTrue(all(item["mcp"] in ("partial", "no") for item in mcp_gaps))

            rc, out, _ = run_cli("show", "AUDIT-085", "--json")
            self.assertEqual(rc, 0)
            shown = json.loads(out)
            self.assertEqual(shown["table"], "bugs")
            self.assertEqual(shown["item"]["status"], "fixed")

            rc, out, _ = run_cli("query", "SELECT id FROM bugs WHERE status != 'fixed'", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [])

            rc, _, err = run_cli("query", "DELETE FROM bugs")
            self.assertNotEqual(rc, 0)
            self.assertTrue(err.strip())
            rc, out, _ = run_cli("query", "SELECT COUNT(*) AS cnt FROM bugs", "--json")
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out), [{"cnt": 98}])

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
            features_toml.write_text(text.replace("notes = \"", "notes = \"Changed: ", 1), encoding="utf-8")
            rc, _, _ = run_cli("check", "--json")
            self.assertEqual(rc, 1)

    def test_sqlite_mutations_roundtrip_toml_and_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            shutil.copytree(REPO_ROOT / "scorecard", root / "scorecard")
            (root / "scorecard" / "scorecard.db").unlink(missing_ok=True)
            shutil.rmtree(root / "scorecard" / ".cache", ignore_errors=True)
            (root / "docs").mkdir(parents=True)

            rc = sdb.main(
                [
                    "update-bug",
                    "AUDIT-085",
                    "--status",
                    "fixed",
                    "--resolution",
                    'Verified CLI & MCP parity with quotes "ok" and slash \\ line1',
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)

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
                    "#99.1",
                    "--area",
                    "cli_mcp",
                    "--severity",
                    "low",
                    "--category",
                    "test_gap",
                    "--target",
                    "cli/vesta-cli/src/lib.rs:1",
                    "--summary",
                    "Synthetic follow-up test item",
                    "--repo-root",
                    str(root),
                ]
            )
            self.assertEqual(rc, 0)

            parsed, errors = load_and_validate(root)
            self.assertEqual(errors, [])
            self.assertEqual(len(parsed["bugs"]["item"]), 99)
            audit_85 = next(b for b in parsed["bugs"]["item"] if b["id"] == "AUDIT-085")
            self.assertEqual(audit_85["status"], "fixed")
            self.assertIn('"ok"', audit_85["resolution"])
            fc_02 = next(f for f in parsed["features"]["item"] if f["id"] == "FC-02")
            self.assertEqual(fc_02["playwright"], "yes")

            out_buf = io.StringIO()
            with contextlib.redirect_stdout(out_buf):
                rc = sdb.main(
                    ["query", "SELECT id, status FROM bugs WHERE id = 'AUDIT-099'", "--json", "--repo-root", str(root)]
                )
            self.assertEqual(rc, 0)
            self.assertEqual(json.loads(out_buf.getvalue()), [{"id": "AUDIT-099", "status": "open"}])

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

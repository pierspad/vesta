#!/usr/bin/env python3
"""Unit tests for build-scripts/generate_scorecard.py and repository scorecard TOMLs."""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

from generate_scorecard import compute_metrics, load_and_validate, main, render_scorecard_md

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
            text = bugs_path.read_text(encoding="utf-8").replace('status = "open"', 'status = "fixed"', 1)
            bugs_path.write_text(text, encoding="utf-8")
            _, errors = load_and_validate(root)
            self.assertTrue(any("requires non-empty 'resolution'" in e for e in errors), errors)

    def test_metrics_scan_and_render_smoke(self) -> None:
        parsed, errors = load_and_validate(REPO_ROOT)
        self.assertEqual(errors, [])
        metrics = compute_metrics(REPO_ROOT)
        self.assertEqual(metrics.workspace_crates, 24)
        self.assertEqual(metrics.ipc_total, 106)
        self.assertEqual(metrics.mcp_tools, 12)
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


if __name__ == "__main__":
    unittest.main()

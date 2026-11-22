#!/usr/bin/env python3
"""Unit tests for build-scripts/browser_ipc_contract.py."""
from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from browser_ipc_contract import (
    ROOT,
    browser_contract,
    check_all,
    check_frontend_invocations,
    check_mock_coverage,
)


class BrowserIpcContractTests(unittest.TestCase):
    def test_repo_contract_checks_pass_and_default_cli_outputs_json(self) -> None:
        contract, errors = check_all(ROOT)
        self.assertEqual(errors, [])
        self.assertIn("parse_subtitle_file", contract)
        self.assertEqual(contract["parse_subtitle_file"]["required"], ["path"])
        self.assertEqual(contract["save_subtitle_file"]["required"], ["path", "entries"])

        stdout = subprocess.check_output(
            ["python3", str(ROOT / "build-scripts/browser_ipc_contract.py")],
            cwd=ROOT,
            text=True,
        )
        parsed = json.loads(stdout)
        self.assertEqual(parsed, contract)

    def test_check_mock_coverage_detects_missing_and_unknown_handlers(self) -> None:
        contract = {
            "ping": {"required": [], "optional": []},
            "save": {"required": ["path"], "optional": []},
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            mock_dir = root / "apps/vesta-gui/browser-tests"
            mock_dir.mkdir(parents=True)
            mock_file = mock_dir / "vesta-mock.mjs"

            mock_file.write_text('switch (command) { case "ping": return 1; case "ghost": return 2; }', encoding="utf-8")
            errors = check_mock_coverage(contract, root)
            self.assertTrue(any("ghost" in e for e in errors), errors)
            self.assertTrue(any("save" in e for e in errors), errors)

            mock_file.write_text('switch (command) { case "ping": return 1; case "save": return 2; }', encoding="utf-8")
            self.assertEqual(check_mock_coverage(contract, root), [])

    def test_check_frontend_invocations_validates_literal_payload_keys(self) -> None:
        contract = {
            "ping": {"required": [], "optional": ["url"]},
            "save_entry": {"required": ["path", "entries"], "optional": ["dryRun"]},
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            src_dir = root / "apps/vesta-gui/src/lib"
            src_dir.mkdir(parents=True)
            ts_file = src_dir / "client.ts"

            # Valid invocations (literal, shorthand, empty args, spread skipped)
            ts_file.write_text(
                "\n".join(
                    [
                        'invokeCommand("ping");',
                        'invokeCommand<number>("ping", { url });',
                        'invokeCommand("save_entry", { path, entries: [{ id: 1 }], dryRun: false });',
                        'invokeCommand("save_entry", { ...dynamicPayload });',
                    ]
                ),
                encoding="utf-8",
            )
            self.assertEqual(check_frontend_invocations(contract, root), [])

            # Unknown key and missing required key caught
            ts_file.write_text(
                'invokeCommand("save_entry", { path: "/a", wrongKey: true });',
                encoding="utf-8",
            )
            errors = check_frontend_invocations(contract, root)
            self.assertTrue(any("unexpected key(s) ['wrongKey']" in e for e in errors), errors)
            self.assertTrue(any("missing required key(s) ['entries']" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()

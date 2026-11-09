#!/usr/bin/env python3
"""Unit tests for build-scripts/check_doc_contracts.py."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_doc_contracts as cdc  # noqa: E402


class TestDocContracts(unittest.TestCase):
    def test_slugify_github(self) -> None:
        self.assertEqual(
            cdc.slugify_github("1. Automated Verification Gates"),
            "1-automated-verification-gates",
        )
        self.assertEqual(
            cdc.slugify_github("`srt-flashcards` — Anki Deck & Media Generator"),
            "srt-flashcards--anki-deck--media-generator",
        )
        self.assertEqual(
            cdc.slugify_github("Step 3: Export (`tsv` / `apkg`)"),
            "step-3-export-tsv--apkg",
        )

    def test_current_repository_contracts_pass(self) -> None:
        manifest, ctx = cdc.build_current_manifest()
        cross_errors = cdc.verify_cross_contracts(
            ctx["tauri_gui"],
            ctx["mcp"],
            ctx["cli"],
            ctx["plugins"],
            ctx["rust_apis"],
        )
        self.assertEqual(cross_errors, [])
        checked_links, link_errors = cdc.verify_markdown_links()
        self.assertGreater(checked_links, 150)
        self.assertEqual(link_errors, [])
        self.assertEqual(ctx["mcp"]["tool_count"], 10)
        self.assertEqual(len(ctx["plugins"]["widget_types"]), 23)

    def test_compare_manifests_detects_micro_and_macro_drift(self) -> None:
        manifest, _ = cdc.build_current_manifest()
        self.assertEqual(cdc.compare_manifests(manifest, manifest), [])

        mutated = copy.deepcopy(manifest)
        mutated["macro_areas"]["tauri_ipc"]["hash"] = "0000000000000000"
        mutated["macro_areas"]["tauri_ipc"]["groups"]["sync"] = "ffffffffffffffff"
        drifts = cdc.compare_manifests(manifest, mutated)
        self.assertEqual(len(drifts), 1)
        self.assertIn("Tauri IPC commands changed in groups ['sync']", drifts[0])

        mutated_rust = copy.deepcopy(manifest)
        mutated_rust["macro_areas"]["rust_libraries"]["hash"] = "1111111111111111"
        mutated_rust["macro_areas"]["rust_libraries"]["crates"]["srt-refine"][
            "hash"
        ] = "2222222222222222"
        drifts_rust = cdc.compare_manifests(manifest, mutated_rust)
        self.assertEqual(len(drifts_rust), 1)
        self.assertIn("crate 'srt-refine'", drifts_rust[0])
        self.assertIn("docs/modules/srt-refine.md", drifts_rust[0])

    def test_verify_cross_contracts_detects_missing_mcp_tool_or_widget(self) -> None:
        _, ctx = cdc.build_current_manifest()
        bad_mcp = copy.deepcopy(ctx["mcp"])
        bad_mcp["tools"]["nonexistent_ghost_tool"] = "deadbeefdeadbeef"
        errors = cdc.verify_cross_contracts(
            ctx["tauri_gui"],
            bad_mcp,
            ctx["cli"],
            ctx["plugins"],
            ctx["rust_apis"],
        )
        self.assertTrue(
            any("nonexistent_ghost_tool" in e and "[MCP Docs]" in e for e in errors)
        )


if __name__ == "__main__":
    unittest.main()

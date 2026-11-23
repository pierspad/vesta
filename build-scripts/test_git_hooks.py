#!/usr/bin/env python3
"""Exercise hook policy against real staged files and outgoing Git commits."""
import json
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SOURCE = Path(__file__).resolve().parents[1]

class GitHookTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="vesta-hooks-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.run_cmd("git", "init", "-q")
        self.run_cmd("git", "config", "user.name", "Hook Test")
        self.run_cmd("git", "config", "user.email", "hooks@example.invalid")
        (self.root / "build-scripts").mkdir()
        for name in ["pre-commit", "pre-push", "install-hooks.sh", "check-i18n.sh", "check-push-i18n.sh",
                     "generate_scorecard.py", "scorecard_db.py"]:
            shutil.copy2(SOURCE / "build-scripts" / name, self.root / "build-scripts" / name)
        scripts = self.root / "apps/vesta-gui/scripts"
        scripts.mkdir(parents=True)
        for name in ["check_missing_translations.py", "check-i18n.mjs", "i18n-scanner.mjs"]:
            shutil.copy2(SOURCE / "apps/vesta-gui/scripts" / name, scripts / name)
        # Source snapshots remain isolated; use the already installed Svelte
        # compiler to exercise the same AST audit as real hooks.
        (self.root / "apps/vesta-gui/node_modules").symlink_to(
            SOURCE / "apps/vesta-gui/node_modules", target_is_directory=True
        )
        self.locales = self.root / "apps/vesta-gui/src/lib/i18n/locales"
        self.locales.mkdir(parents=True)
        for file in (SOURCE / "apps/vesta-gui/src/lib/i18n/locales").glob("*.json"):
            self.write_locale(file.stem, "Original English text" if file.stem == "en" else f"Translation {file.stem}")
        shutil.copytree(SOURCE / "scorecard", self.root / "scorecard")
        (self.root / "docs").mkdir()
        self.run_cmd("python3", "build-scripts/generate_scorecard.py")
        (self.root / ".gitignore").write_text("node_modules\n")
        self.run_cmd("git", "add", ".")
        self.run_cmd("git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "fixture")

    def run_cmd(self, *args, input=None, ok=True):
        result = subprocess.run(args, cwd=self.root, input=input, text=True, capture_output=True)
        if ok:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def write_locale(self, locale, text):
        (self.locales / f"{locale}.json").write_text(json.dumps({"test.label": text}) + "\n")

    def test_staged_english_is_blocked_even_when_worktree_is_fixed(self):
        self.write_locale("fr", "Original English text")
        self.run_cmd("git", "add", ".")
        self.write_locale("fr", "Texte français corrigé")
        result = self.run_cmd("bash", "build-scripts/pre-commit", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("same_as_english", result.stdout)
        self.run_cmd("git", "add", ".")
        self.run_cmd("bash", "build-scripts/pre-commit")

    def test_outgoing_bad_commit_is_blocked_despite_uncommitted_fix(self):
        self.write_locale("fr", "Original English text")
        self.run_cmd("git", "add", ".")
        self.run_cmd("git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "untranslated")
        revision = self.run_cmd("git", "rev-parse", "HEAD").stdout.strip()
        self.write_locale("fr", "Texte français corrigé")
        result = self.run_cmd("bash", "build-scripts/check-push-i18n.sh", input=f"refs/heads/dev {revision} refs/heads/dev {'0' * 40}\n", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("same_as_english", result.stdout)

    def test_staged_source_key_is_checked(self):
        (self.root / "apps/vesta-gui/src/Example.svelte").write_text('<p>{t("missing.key")}</p>')
        self.run_cmd("git", "add", ".")
        result = self.run_cmd("bash", "build-scripts/pre-commit", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing.key", result.stdout + result.stderr)

    def test_staged_conditional_copy_uses_the_compiler_even_when_worktree_is_fixed(self):
        source = self.root / "apps/vesta-gui/src/Example.svelte"
        source.write_text('<button title={open ? "Collapse sidebar" : "Expand sidebar"}>{t("test.label")}</button>')
        self.run_cmd("git", "add", ".")
        source.write_text('<button title={t("test.label")}>{t("test.label")}</button>')
        result = self.run_cmd("bash", "build-scripts/pre-commit")
        self.assertIn('possible hardcoded text "Collapse sidebar"', result.stdout + result.stderr)
        self.assertIn('possible hardcoded text "Expand sidebar"', result.stdout + result.stderr)

    def test_install_is_idempotent_and_dispatches_current_script(self):
        self.run_cmd("bash", "build-scripts/install-hooks.sh")
        self.run_cmd("bash", "build-scripts/install-hooks.sh")
        (self.root / "build-scripts/pre-commit").write_text("#!/bin/bash\nexit 42\n")
        result = self.run_cmd("bash", ".git/hooks/pre-commit", ok=False)
        self.assertEqual(result.returncode, 42)

    def test_custom_hook_path_is_respected_and_unrelated_hooks_survive(self):
        self.run_cmd("git", "config", "core.hooksPath", "custom-hooks")
        self.run_cmd("bash", "build-scripts/install-hooks.sh")
        hook = self.root / "custom-hooks/pre-commit"
        self.assertTrue(hook.is_file())
        hook.write_text("#!/bin/bash\n# personal hook\nexit 0\n")
        result = self.run_cmd("bash", "build-scripts/install-hooks.sh", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("personal hook", hook.read_text())

    def test_pre_push_uses_locked_and_excludes_desktop_without_webkit(self):
        self.run_cmd("bash", "-n", "build-scripts/pre-push")
        text = (self.root / "build-scripts/pre-push").read_text(encoding="utf-8")
        self.assertIn("--locked --workspace", text)
        self.assertIn("webkit2gtk-4.1", text)
        self.assertIn("--exclude vesta --exclude whisper-bench", text)

    def test_staged_source_metrics_require_a_staged_dashboard_update(self):
        source = self.root / "apps/vesta-gui/src/example.test.ts"
        source.write_text('test("new case", () => {});\n')
        self.run_cmd("git", "add", ".")
        # A generated but unstaged dashboard must not conceal stale index data.
        self.run_cmd("python3", "build-scripts/generate_scorecard.py")
        result = self.run_cmd("bash", "build-scripts/pre-commit", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("docs/SCORECARD.md is stale", result.stderr)
        self.run_cmd("git", "add", "docs/SCORECARD.md")
        # Unstaged source must likewise not influence the index's metrics.
        source.write_text(source.read_text() + 'test("unstaged case", () => {});\n')
        self.run_cmd("bash", "build-scripts/pre-commit")

    def test_staged_scorecard_database_drift_is_blocked(self):
        features = self.root / "scorecard/features.toml"
        features.write_text(features.read_text().replace('notes = "', 'notes = "Updated ', 1))
        self.run_cmd("python3", "build-scripts/generate_scorecard.py")
        self.run_cmd("git", "add", ".")
        result = self.run_cmd("bash", "build-scripts/pre-commit", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("scorecard.db", result.stdout + result.stderr)
        self.run_cmd("python3", "build-scripts/scorecard_db.py", "sync")
        self.run_cmd("git", "add", "scorecard/scorecard.db")
        self.run_cmd("bash", "build-scripts/pre-commit")

    def test_outgoing_stale_dashboard_is_blocked_despite_uncommitted_fix(self):
        (self.root / "apps/vesta-gui/src/example.test.ts").write_text('test("new case", () => {});\n')
        self.run_cmd("git", "add", ".")
        self.run_cmd("git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "stale metrics")
        revision = self.run_cmd("git", "rev-parse", "HEAD").stdout.strip()
        self.run_cmd("python3", "build-scripts/generate_scorecard.py")
        result = self.run_cmd("bash", "build-scripts/check-push-i18n.sh",
                              input=f"refs/heads/dev {revision} refs/heads/dev {'0' * 40}\n", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("docs/SCORECARD.md is stale", result.stderr)
        self.run_cmd("git", "add", "docs/SCORECARD.md")
        self.run_cmd("git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "refresh metrics")
        revision = self.run_cmd("git", "rev-parse", "HEAD").stdout.strip()
        # Pushing one commit to both branches should check the snapshot only once.
        self.run_cmd("bash", "build-scripts/check-push-i18n.sh",
                     input=''.join(f"refs/heads/dev {revision} refs/heads/{branch} {'0' * 40}\n"
                                   for branch in ("dev", "main")))

if __name__ == "__main__":
    unittest.main()

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
        for name in ["pre-commit", "pre-push", "install-hooks.sh", "check-i18n.sh", "check-push-i18n.sh"]:
            shutil.copy2(SOURCE / "build-scripts" / name, self.root / "build-scripts" / name)
        scripts = self.root / "apps/srt-gui/scripts"
        scripts.mkdir(parents=True)
        for name in ["check_missing_translations.py", "check-i18n.mjs", "i18n-scanner.mjs"]:
            shutil.copy2(SOURCE / "apps/srt-gui/scripts" / name, scripts / name)
        # Source snapshots remain isolated; use the already installed Svelte
        # compiler to exercise the same AST audit as real hooks.
        (self.root / "apps/srt-gui/node_modules").symlink_to(
            SOURCE / "apps/srt-gui/node_modules", target_is_directory=True
        )
        self.locales = self.root / "apps/srt-gui/src/lib/i18n/locales"
        self.locales.mkdir(parents=True)
        for file in (SOURCE / "apps/srt-gui/src/lib/i18n/locales").glob("*.json"):
            self.write_locale(file.stem, "Original English text" if file.stem == "en" else f"Translation {file.stem}")
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
        (self.root / "apps/srt-gui/src/Example.svelte").write_text('<p>{t("missing.key")}</p>')
        self.run_cmd("git", "add", ".")
        result = self.run_cmd("bash", "build-scripts/pre-commit", ok=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("missing.key", result.stdout + result.stderr)

    def test_staged_conditional_copy_uses_the_compiler_even_when_worktree_is_fixed(self):
        source = self.root / "apps/srt-gui/src/Example.svelte"
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

if __name__ == "__main__":
    unittest.main()

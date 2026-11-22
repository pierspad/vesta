import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { checkUiContracts, isAllowedLiteralAttr } from "./check-ui-contracts.mjs";

function makeTempProject() {
  const dir = mkdtempSync(join(tmpdir(), "vesta-ui-contracts-"));
  mkdirSync(join(dir, "lib/utils"), { recursive: true });
  mkdirSync(join(dir, "lib/components"), { recursive: true });
  mkdirSync(join(dir, "lib/tabs"), { recursive: true });
  return dir;
}

test("isAllowedLiteralAttr accepts numeric increments and rejects prose literals", () => {
  assert.equal(isAllowedLiteralAttr("-3s"), true);
  assert.equal(isAllowedLiteralAttr("X (%)"), true);
  assert.equal(isAllowedLiteralAttr("https://example.com"), true);
  assert.equal(isAllowedLiteralAttr("Open subtitle file"), false);
});

test("checkUiContracts flags unhandled actions, duplicate keys, hardcoded tooltips, and unlabelled buttons", () => {
  const srcDir = makeTempProject();
  try {
    writeFileSync(
      join(srcDir, "lib/utils/shortcuts.ts"),
      `export const defaultShortcuts = [
        { id: "show-help", action: "showShortcutHelp", description: "shortcuts.action.showHelp", defaultKey: "Shift+F1", category: "global" },
        { id: "sync-a", action: "doSyncA", description: "shortcuts.action.a", defaultKey: "Ctrl+A", category: "sync" },
        { id: "sync-b", action: "doSyncB", description: "shortcuts.action.missing", defaultKey: "Ctrl+A", category: "sync" },
        { id: "transcribe-open-file", action: "openInputFile", description: "shortcuts.action.openSrt", defaultKey: "Ctrl+O", category: "transcribe" },
      ];`,
    );
    writeFileSync(join(srcDir, "lib/components/ShortcutOverlay.svelte"), `tabToCategory = { sync: "sync" }; // F1`);
    writeFileSync(join(srcDir, "lib/tabs/ShortcutsTab.svelte"), `categoryLabels = { sync: "sync" };`);
    writeFileSync(
      join(srcDir, "App.svelte"),
      `<script>const x = "doSyncA"; const y = "openInputFile";</script>
      <button title="Hardcoded tooltip">Ok</button>
      <button><svg /></button>`,
    );
    writeFileSync(
      join(srcDir, "lib/tabs/SampleTab.svelte"),
      `<button title="Hardcoded tooltip">Ok</button>
      <button><svg></svg></button>`,
    );

    const result = checkUiContracts({
      srcDir,
      enCatalog: {
        "shortcuts.action.showHelp": "Show help",
        "shortcuts.action.a": "Action A",
        "shortcuts.action.openSrt": "Open SRT file",
      },
    });
    assert.ok(result.errors.some((e) => e.includes('missing i18n description key "shortcuts.action.missing"')));
    assert.ok(result.errors.some((e) => e.includes('action "doSyncB" has no handler')));
    assert.ok(result.errors.some((e) => e.includes('Duplicate defaultKey "Ctrl+A"')));
    assert.ok(result.errors.some((e) => e.includes("transcribe category wrongly describes opening SRT")));
    assert.ok(result.errors.some((e) => e.includes('hardcoded literal title="Hardcoded tooltip"')));
    assert.ok(result.errors.some((e) => e.includes("icon-only <button> is missing both title and aria-label")));
  } finally {
    rmSync(srcDir, { recursive: true, force: true });
  }
});

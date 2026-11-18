import { expect, test } from "./browser-fixture.mjs";

const fixturePath = "/fixtures/Episode01.ja.srt";
const callsFor = (page, command) => page.evaluate(
  name => window.browserFixture.calls.filter(call => call.command === name).map(call => call.args),
  command,
);
const chooseFile = async (page, path, button) => {
  await page.evaluate(value => window.browserFixture.selectFile(value), path);
  await button.click();
};

test.beforeEach(async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await expect(page.getByRole("navigation")).toBeVisible();
});

test("Sync updates its anchor through undo, redo and removal", async ({ page }) => {
  await page.route("http://127.0.0.1:12345/media**", route => {
    const wave = Buffer.alloc(44 + 48000);
    wave.write("RIFF", 0);
    wave.writeUInt32LE(wave.length - 8, 4);
    wave.write("WAVEfmt ", 8);
    wave.writeUInt32LE(16, 16);
    wave.writeUInt16LE(1, 20);
    wave.writeUInt16LE(1, 22);
    wave.writeUInt32LE(8000, 24);
    wave.writeUInt32LE(16000, 28);
    wave.writeUInt16LE(2, 32);
    wave.writeUInt16LE(16, 34);
    wave.write("data", 36);
    wave.writeUInt32LE(wave.length - 44, 40);
    return route.fulfill({ contentType: "audio/wav", body: wave, headers: { "access-control-allow-origin": "*" } });
  });
  await page.getByRole("button", { name: "Synchronize", exact: true }).click();
  await chooseFile(page, fixturePath, page.getByRole("button", { name: "Load SRT subtitle file" }));
  await expect.poll(() => callsFor(page, "sync_load_srt")).toEqual([{ path: fixturePath }]);
  const browseMedia = page.getByRole("button", { name: "Load video file", exact: true });
  await page.evaluate(() => window.browserFixture.failMedia("Unreadable media fixture"));
  await chooseFile(page, "/fixtures/audio.wav", browseMedia);
  await expect(page.getByRole("alert").filter({ hasText: "Unreadable media fixture" })).toBeVisible();
  await page.evaluate(() => window.browserFixture.failMedia(null));
  await chooseFile(page, "/fixtures/audio.wav", browseMedia);
  await expect.poll(() => callsFor(page, "sync_set_video")).toEqual([{ path: "/fixtures/audio.wav" }]);

  const confirm = page.getByRole("button", { name: "Confirm anchor", exact: true });
  await page.getByRole("button", { name: "+0.5s", exact: true }).click();
  await confirm.click();
  await expect.poll(() => callsFor(page, "sync_add_anchor")).toEqual([{ subtitleId: 1, correctedTimeMs: 1500 }]);
  const remove = page.getByRole("button", { name: "Remove this sync anchor", exact: true });
  const subtitle = page.getByRole("button", { name: /#1 Synthetic subtitle/ });
  await expect(subtitle).toContainText("0:01.500");
  await expect(remove).toBeVisible();
  await page.keyboard.press("Control+z");
  await expect.poll(() => callsFor(page, "sync_undo")).toHaveLength(1);
  await expect(remove).toBeHidden();
  await expect(subtitle).toContainText("0:01.000");
  await expect(page.getByRole("button", { name: "Redo", exact: true })).toBeEnabled();
  await page.keyboard.press("Control+Shift+z");
  await expect.poll(() => callsFor(page, "sync_redo")).toHaveLength(1);
  await expect(subtitle).toContainText("0:01.500");
  await remove.click();
  await expect.poll(() => callsFor(page, "sync_remove_anchor")).toHaveLength(1);
  await expect(remove).toBeHidden();
});

test("Align restores both editors after swapping tracks", async ({ page }) => {
  await page.getByRole("button", { name: "Revise", exact: true }).click();
  const picker = page.getByRole("button", { name: "Open SRT", exact: true });
  await chooseFile(page, fixturePath, picker.first());
  await chooseFile(page, "/fixtures/reference.srt", picker.last());
  const first = page.locator("textarea:visible").first();
  await expect(first).toHaveValue("Synthetic subtitle for the browser test");
  await page.getByRole("button", { name: /Swap/ }).click();
  await expect(first).toHaveValue("Reference subtitle");
  await page.keyboard.press("Control+z");
  await expect(first).toHaveValue("Synthetic subtitle for the browser test");
  await page.keyboard.press("Control+Shift+z");
  await expect(first).toHaveValue("Reference subtitle");
});

test("Translate reports a provider error and cancels a retry", async ({ page }) => {
  await page.getByRole("button", { name: "Translate", exact: true }).click();
  await chooseFile(page, fixturePath, page.getByRole("button", { name: "Select input SRT file", exact: true }));
  const expert = page.getByRole("switch", { name: "Expert Mode" });
  await expert.click();
  await expect(expert).toBeChecked();
  await expert.click();
  await expect(expert).not.toBeChecked();

  const start = page.getByRole("button", { name: "Start Translation", exact: true });
  await start.click();
  await expect.poll(() => callsFor(page, "start_translation")).toHaveLength(1);
  await page.evaluate(() => window.browserFixture.failTranslation("Fixture provider unavailable"));
  await expect(page.locator("[data-footer-actions]").getByText("Translation error: Fixture provider unavailable", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Retry Translation", exact: true }).click();
  await expect.poll(() => callsFor(page, "start_translation")).toHaveLength(2);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect.poll(() => callsFor(page, "cancel_translation")).toHaveLength(1);
  await expect(page.getByRole("button", { name: "Cancel", exact: true })).toBeHidden();
});

test("Transcribe uses the selected model and cancels its run", async ({ page }) => {
  await page.getByRole("button", { name: "Transcribe", exact: true }).click();
  await chooseFile(page, "/fixtures/audio.wav", page.getByRole("button", { name: "Select file...", exact: true }));
  await page.getByRole("combobox", { name: "Transcription engine" }).click();
  await page.getByRole("option", { name: /Whisper.*small/i }).click();
  const start = page.getByRole("button", { name: "Start Transcription", exact: true });
  await start.click();
  await expect.poll(() => callsFor(page, "transcribe_start")).toMatchObject([{ config: { model: "small" } }]);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect.poll(() => callsFor(page, "transcribe_cancel")).toHaveLength(1);
  await expect(start).toBeVisible();
});

test("Transcribe advances to the next provider after a failed attempt", async ({ page }) => {
  await page.getByRole("button", { name: "Transcribe", exact: true }).click();
  await chooseFile(page, "/fixtures/audio.wav", page.getByRole("button", { name: "Select file...", exact: true }));
  await page.getByRole("button", { name: "Start Transcription", exact: true }).click();
  await expect.poll(() => callsFor(page, "transcribe_start")).toMatchObject([{ config: { model: "base" } }]);
  await page.evaluate(() => window.browserFixture.failTranscription("Fixture model failed"));
  await expect.poll(() => callsFor(page, "transcribe_start")).toMatchObject([
    { config: { model: "base" } },
    { config: { provider: "groq", model: "whisper-large-v3" } },
  ]);
  await page.evaluate(() => window.browserFixture.completeTranscription());
  await expect(page.getByText("output.srt", { exact: true })).toBeVisible();
});

test("Settings shortcuts detect conflicts and persist a new binding", async ({ page }) => {
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Shortcuts", exact: true }).click();
  const textSearch = page.getByPlaceholder("Search by text...");
  await textSearch.fill("Preview cards");
  const action = page.getByText("Preview cards", { exact: true });
  await expect(action).toBeVisible();
  await action.click();
  await page.keyboard.press("Control+Enter");
  await expect(page.getByRole("alert").filter({ hasText: "Conflict" })).toBeVisible();
  await action.click();
  await page.keyboard.press("Control+Alt+y");
  await expect.poll(() => callsFor(page, "config_set")).toEqual(expect.arrayContaining([
    { key: "srt-tools-shortcut-overrides", value: JSON.stringify({ "flashcards-preview": "Ctrl+Alt+Y" }) },
  ]));
  await page.getByRole("button", { name: "KEYS", exact: true }).click();
  const recorder = page.getByRole("button", { name: "Search by keys...", exact: true });
  await expect(page.getByText("Generate flashcards", { exact: true })).toBeVisible();
  await recorder.press("Enter");
  await page.keyboard.press("Control+Alt+y");
  await expect(action).toBeVisible();
  await expect(page.getByText("Generate flashcards", { exact: true })).toBeHidden();
});

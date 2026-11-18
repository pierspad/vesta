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

test("Extract previews and runs OCR on a bitmap PGS track", async ({ page }) => {
  await page.getByRole("button", { name: "Extract subtitles", exact: true }).click();
  await chooseFile(page, "/fixtures/movie.mkv", page.getByRole("button", { name: "Browse", exact: true }).first());

  const trackCard = page.getByRole("group", { name: "English" });
  await expect(trackCard).toContainText("hdmv_pgs_subtitle");

  await trackCard.getByRole("button", { name: /View subtitles/ }).click();
  const dialog = page.getByRole("dialog", { name: /View subtitles/ });
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("textarea")).toHaveValue(/Synthetic OCR preview/);
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(dialog).toBeHidden();

  await trackCard.getByRole("button", { name: /Save SRT/ }).click();
  await expect.poll(() => callsFor(page, "ocr_embedded_subtitle")).toEqual([{
    path: "/fixtures/movie.mkv",
    index: 2,
    outputPath: "/downloads/movie_en_2.srt",
    language: "en",
    tiers: [[{
      provider: "google",
      model: "gemini-fixture",
      api_key: "fixture-key",
      api_url: "https://generativelanguage.googleapis.com/v1beta",
      rpm: null,
      max_requests: null,
    }]],
  }]);
  await expect(trackCard.getByLabel("Already downloaded")).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "Saved: /downloads/movie_en_2.srt" })).toBeVisible();
});

test("Extract previews and runs hardcoded ROI OCR on video frames", async ({ page }) => {
  await page.getByRole("button", { name: "Extract subtitles", exact: true }).click();
  await chooseFile(page, "/fixtures/movie.mkv", page.getByRole("button", { name: "Browse", exact: true }).first());

  const roiPanel = page.locator("#hardcoded-roi-panel");
  await expect(roiPanel).toBeVisible();
  await roiPanel.getByRole("button", { name: /Hardcoded subtitles \(ROI OCR\)/ }).click();
  await expect.poll(() => callsFor(page, "ocr_video_preview_frame")).toHaveLength(1);

  await roiPanel.getByRole("button", { name: "Preview ROI OCR", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: /Hardcoded subtitles/ });
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("textarea")).toHaveValue(/Hardcoded subtitle line 1/);
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(dialog).toBeHidden();

  await roiPanel.getByRole("button", { name: "Extract ROI to SRT", exact: true }).click();
  await expect.poll(() => callsFor(page, "ocr_hardcoded_subtitle")).toMatchObject([{
    path: "/fixtures/movie.mkv",
    outputPath: "/downloads/movie_hardcoded.srt",
    roi: { x: 0.08, y: 0.74, width: 0.84, height: 0.2 },
    sampleFps: 2.5,
  }]);
  await expect(page.getByRole("status").filter({ hasText: "Saved: /downloads/movie_hardcoded.srt" })).toBeVisible();
});

test("Refine loads cards, annotates manually and with AI, and saves changes", async ({ page }) => {
  await page.getByRole("button", { name: "Annotate", exact: true }).click();
  await chooseFile(page, "/fixtures/deck.tsv", page.getByRole("button", { name: "Browse flashcard file", exact: true }));
  await expect.poll(() => callsFor(page, "refine_load_file")).toEqual([{ path: "/fixtures/deck.tsv" }]);

  const notes = page.locator("#card-notes");
  await expect(notes).toBeVisible();
  await notes.fill("Manual draft note");
  await notes.blur();
  await expect(notes).toHaveValue("Manual draft note");

  await page.getByRole("button", { name: "Annotate with AI", exact: true }).click();
  await expect.poll(() => callsFor(page, "refine_card_llm_tiered")).toHaveLength(1);
  await expect(notes).toHaveValue("Grammar note from AI");

  await page.getByRole("button", { name: /Overwrite TSV/ }).click();
  await expect.poll(() => callsFor(page, "refine_save_file")).toEqual([{
    inputPath: "/fixtures/deck.tsv",
    outputPath: "/fixtures/deck.tsv",
    updates: [
      { id: "card-1", notes: "Grammar note from AI" },
      { id: "card-2", notes: "Existing note" },
    ],
  }]);
});

test("Plugins inspects, installs, opens, dispatches actions, and uninstalls a plugin", async ({ page }) => {
  await page.getByRole("button", { name: "Plugins", exact: true }).click();
  await page.getByPlaceholder("https://github.com/owner/repository").fill("https://github.com/vesta/demo-plugin");
  await page.getByRole("button", { name: "Install Plugin", exact: true }).click();

  const dialog = page.locator("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("Demo Study Plugin");
  await dialog.getByRole("button", { name: "Confirm & Install", exact: true }).click();
  await expect.poll(() => callsFor(page, "plugin_install")).toEqual([{
    url: "https://github.com/vesta/demo-plugin",
    gitRef: undefined,
  }]);
  await expect(dialog).toBeHidden();

  await page.getByRole("button", { name: "Open", exact: true }).click();
  await expect(page.getByText("Demo Study Workspace", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Run Check", exact: true }).click();
  await expect.poll(() => callsFor(page, "plugin_dispatch_event")).toHaveLength(1);
  await expect(page.getByText("Plugin action completed: 2 items verified.", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Back", exact: true }).click();
  await page.getByRole("button", { name: "Uninstall plugin", exact: true }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect.poll(() => callsFor(page, "plugin_uninstall")).toEqual([{ id: "com.vesta.demo-plugin" }]);
});

test("Settings runs diagnostics, downloads offline OCR model, and toggles AI Kill Switch", async ({ page }) => {
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Diagnostics", exact: true }).click();

  await expect(page.getByText("libx264", { exact: true })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);

  await page.getByRole("button", { name: "Record", exact: true }).click();
  await expect.poll(() => callsFor(page, "support_log_start")).toHaveLength(1);
  await expect(page.locator("#support-log-path")).toHaveValue("/exports/vesta-support.jsonl");
  await page.getByRole("button", { name: "Stop recording", exact: true }).click();
  await expect.poll(() => callsFor(page, "support_log_stop")).toHaveLength(1);

  await page.getByRole("button", { name: "STT", exact: true }).click();
  const ocrSection = page.locator("#ocr-models");
  await expect(ocrSection).toBeVisible();
  await expect(ocrSection).toContainText("0/3");
  await ocrSection.getByRole("radio", { name: /PaddleOCR v4 Mobile \(Latin\)/ }).click();
  await expect.poll(() => callsFor(page, "ocr_download_model")).toEqual([{ modelId: "paddleocr-v4-mobile-latin" }]);
  await expect(ocrSection).toContainText("1/3");

  const killSwitch = page.getByRole("switch", { name: "AI Kill Switch", exact: true });
  await killSwitch.click();
  await expect(killSwitch).toHaveAttribute("aria-checked", "true");
  await expect(page.getByRole("button", { name: "LLM", exact: true })).toBeHidden();
  await expect(page.getByRole("button", { name: "STT", exact: true })).toBeHidden();

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("button", { name: "Translate", exact: true })).toBeHidden();
  await expect(page.getByRole("button", { name: "Transcribe", exact: true })).toBeHidden();

  await killSwitch.click();
  await expect(killSwitch).toHaveAttribute("aria-checked", "false");
  await expect(page.getByRole("button", { name: "Translate", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Transcribe", exact: true })).toBeVisible();
});

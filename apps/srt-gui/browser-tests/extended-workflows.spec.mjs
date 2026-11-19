import { test, expect } from "./browser-fixture.mjs";

const called = async (page, command) =>
  page.evaluate(
    (cmd) => window.browserFixture.calls.filter((entry) => entry.command === cmd),
    command,
  );

const silentWavBuffer = () => {
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
  return wave;
};

test("Extract previews and extracts an embedded text subtitle track", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.evaluate(() => {
    window.browserFixture.setSubtitleTracks([
      {
        index: 3,
        codec: "subrip",
        language: "eng",
        title: "English UTF-8",
        text_based: true,
      },
    ]);
  });

  await page.getByRole("button", { name: "Extract subtitles", exact: true }).click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/movie.mkv"));
  await page.getByRole("button", { name: "Browse", exact: true }).first().click();

  const trackCard = page.getByRole("group", { name: /English/i });
  await expect(trackCard).toContainText("English UTF-8");

  await trackCard.getByRole("button", { name: /View subtitles/ }).click();
  const dialog = page.getByRole("dialog", { name: /View subtitles/ });
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("textarea")).toHaveValue(/Embedded text track preview/);
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(dialog).toBeHidden();

  await trackCard.getByRole("button", { name: /Save SRT/ }).click();
  await expect.poll(async () => (await called(page, "preview_embedded_subtitle")).length).toBeGreaterThan(0);
  await expect.poll(async () => (await called(page, "extract_embedded_subtitle")).length).toBe(1);
});

test("Sync runs Auto-Sync, saves file, loads session, and resets", async ({ page }) => {
  await page.route("http://127.0.0.1:12345/media**", (route) =>
    route.fulfill({
      contentType: "audio/wav",
      body: silentWavBuffer(),
      headers: { "access-control-allow-origin": "*" },
    }),
  );

  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: "Synchronize", exact: true }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.ja.srt"));
  await page.getByRole("button", { name: "Load SRT subtitle file", exact: true }).click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/audio.wav"));
  await page.getByRole("button", { name: "Load video file", exact: true }).click();

  const autoSyncBtn = page.getByRole("button", { name: "Auto-Sync", exact: true });
  await expect(autoSyncBtn).toBeEnabled();
  await autoSyncBtn.click();
  await expect.poll(async () => (await called(page, "sync_auto_sync")).length).toBe(1);

  await page.getByRole("button", { name: "Save File" }).click();
  await expect.poll(async () => (await called(page, "sync_save_file")).length).toBe(1);

  const saveSessionBtn = page.getByRole("button", { name: "Save Session" });
  if (await saveSessionBtn.isVisible().catch(() => false)) {
    await saveSessionBtn.click();
    await expect.poll(async () => (await called(page, "sync_save_session")).length).toBe(1);
  }

  await page.getByRole("button", { name: "Load Session" }).click();
  await expect.poll(async () => (await called(page, "sync_load_session")).length).toBe(1);

  await page.getByRole("button", { name: /New Sync/i }).click();
  const confirmResetBtn = page.getByRole("dialog").getByRole("button").last();
  await expect(confirmResetBtn).toBeVisible();
  await confirmResetBtn.click();
  await expect.poll(async () => (await called(page, "sync_reset")).length).toBe(1);
});

test("Translate suggests context, closes PathPreviewModal with Escape (#14.7), and completes", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.getByRole("button", { name: "Translate", exact: true }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.ja.srt"));
  await page.getByRole("button", { name: "Select input SRT file" }).click();

  // Regression test for #14.7: Escape must close PathPreviewModal even when focus is inside the card
  const pathButton = page.locator("button[title='/fixtures/Episode01.ja.srt']");
  await expect(pathButton).toBeVisible();
  await pathButton.click();
  const pathDialog = page.getByRole("dialog");
  await expect(pathDialog).toBeVisible();
  await pathDialog.locator("code, pre, p").first().click();
  await page.keyboard.press("Escape");
  await expect(pathDialog).toBeHidden();

  // Suggest context flow (#14.5 guard)
  const suggestBtn = page.getByRole("button", { name: "Suggest context" });
  await expect(suggestBtn).toBeVisible();
  await suggestBtn.click();
  await expect(page.getByRole("button", { name: "Apply draft" })).toBeVisible();
  await page.getByRole("button", { name: "Apply draft" }).click();
  await expect(page.locator("#context-input")).toHaveValue(/Conversational drama dialogue context/);
  await expect.poll(async () => (await called(page, "suggest_translation_context")).length).toBe(1);

  // Select destination & start + complete translation
  await page.evaluate(() => window.browserFixture.selectFile("/exports/Episode01.en.srt"));
  await page.getByRole("button", { name: "Choose the name and location of the translated file" }).click();
  await page.getByRole("button", { name: "Start Translation" }).click();
  await page.evaluate(() => window.browserFixture.completeTranslation());

  await expect.poll(async () => (await called(page, "start_translation")).length).toBe(1);
});

test("Refine runs tiered AI refinement with progress events and summary (#15.2)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: /Annotate/ }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/deck.apkg"));
  await page.getByRole("button", { name: "Browse flashcard file" }).click();
  await expect.poll(async () => (await called(page, "refine_load_file")).length).toBe(1);

  await page.getByRole("button", { name: "Automatic (AI)" }).click();
  await page.getByRole("button", { name: "Start AI Refinement" }).click();

  await expect.poll(async () => (await called(page, "refine_cards_llm_tiered")).length).toBe(1);
  await expect(page.getByRole("status").filter({ hasText: "2" }).first()).toBeVisible();
});

test("Flashcards previews audio and snapshot on loaded video media", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: "Upload media" }).click();

  await expect.poll(async () => (await called(page, "flashcard_media_duration")).length).toBeGreaterThan(0);
  await expect.poll(async () => (await called(page, "flashcard_preview_snapshot")).length).toBeGreaterThan(0);

  const playPreviewBtn = page.getByRole("button", { name: /^Play$/ });
  await expect(playPreviewBtn).toBeEnabled();
  await playPreviewBtn.click();
  await expect.poll(async () => (await called(page, "flashcard_preview_audio")).length).toBeGreaterThan(0);
});

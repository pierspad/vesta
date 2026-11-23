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

test("Translate supports overlap offset tuning, live checkpoint preview, and partial retry resume (TL-02)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.getByRole("button", { name: "Translate", exact: true }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.ja.srt"));
  await page.getByRole("button", { name: "Select input SRT file" }).click();

  const overlapSlider = page.locator("input[type='range'][max='15']").first();
  await overlapSlider.fill("4");
  await overlapSlider.dispatchEvent("change");

  await page.evaluate(() => window.browserFixture.selectFile("/exports/Episode01.en.srt"));
  await page.getByRole("button", { name: "Choose the name and location of the translated file" }).click();

  await page.getByRole("button", { name: "Start Translation" }).click();
  await expect.poll(async () => (await called(page, "start_translation")).length).toBe(1);

  await page.evaluate(() => {
    window.browserFixture.setLatestTranslatedSubtitles([{ id: 1, original: "こんにちは", translated: "Hello" }]);
    window.browserFixture.emitTranslateProgress({
      message: "Batch 1/2",
      current_batch: 1,
      total_batches: 2,
      percentage: 50,
      eta_seconds: 5,
    });
    window.browserFixture.completeTranslationWith({
      success: false,
      message: "Translation incomplete: 1 subtitle remains",
      output_path: "/exports/Episode01.en.srt",
      translated_count: 1,
    });
  });

  const retryBtn = page.getByRole("button", { name: "Retry Translation", exact: true });
  await expect(retryBtn).toBeVisible();
  await retryBtn.click();
  await expect.poll(async () => (await called(page, "start_translation")).length).toBe(2);

  await page.evaluate(() => window.browserFixture.completeTranslation());
  const translateCalls = await called(page, "start_translation");
  expect(translateCalls[0].args.config.output_path).toBe("/exports/Episode01.en.srt");
  expect(translateCalls[1].args.config.output_path).toBe("/exports/Episode01.en.srt");
  expect(translateCalls[1].args.config.resume_overlap).toBe(4);
});

test("Transcribe runs with Silero VAD, quality mode, live segments, and shows disabled VAD prompt when uninstalled (TR-02, TR-03)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.getByRole("button", { name: "Transcribe", exact: true }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.mp4"));
  await page.getByRole("button", { name: "Select file...", exact: true }).click();

  const qualitySwitch = page.getByRole("switch", { name: "Quality mode (beam search)" });
  const vadSwitch = page.getByRole("switch", { name: "Voice Activity Detection" });
  await expect(vadSwitch).toBeEnabled();
  await qualitySwitch.click();

  await page.getByRole("button", { name: "Start Transcription", exact: true }).click();
  await expect.poll(async () => (await called(page, "transcribe_start")).length).toBe(1);
  const startCall = (await called(page, "transcribe_start"))[0];
  expect(startCall.args.config).toMatchObject({
    vad: true,
    vad_model_id: "v6.2.0",
    quality: true,
  });

  await page.evaluate(() => {
    window.browserFixture.emitTranscribeProgress({
      stage: "transcribe",
      message: "Transcribing segment 1",
      percentage: 60,
    });
    window.browserFixture.emitTranscribeSegment({
      start_ms: 1000,
      end_ms: 3500,
      text: "Synthetic live transcribed line",
    });
  });
  await expect(page.getByText("Synthetic live transcribed line", { exact: true })).toBeVisible();
  await page.evaluate(() => window.browserFixture.completeTranscription());

  // Uninstall Silero VAD in Settings -> STT and select a missing custom VAD path so Transcribe disables VAD + links to Settings
  await page.locator("aside > div button").first().click();
  await page.getByRole("button", { name: "STT" }).click();
  await page.locator("#silero-vad").getByRole("button", { name: "Uninstall", exact: true }).click();
  await expect.poll(async () => (await called(page, "transcribe_uninstall_vad")).length).toBe(1);
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/missing-vad.bin"));
  await page.getByRole("button", { name: "Load custom model…" }).click();
  await expect.poll(async () => (await called(page, "transcribe_path_exists")).length).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Transcribe", exact: true }).click();
  await expect(page.getByRole("switch", { name: "Voice Activity Detection" })).toBeDisabled();
  await expect(page.getByText(/Download the Silero Voice Activity Detection model/).first()).toBeVisible();
});

test("Sync supports cloud-guided Auto-Sync, cancellation overlay, and multi-anchor offset spread (AS-02, AS-03)", async ({ page }) => {
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

  const engineCombo = page.getByRole("combobox", { name: "Transcription engine" });
  await engineCombo.click();
  await page.getByRole("option", { name: /Groq/i }).click();

  const autoSyncBtn = page.getByRole("button", { name: "Auto-Sync", exact: true });
  await autoSyncBtn.click();
  await expect.poll(async () => (await called(page, "sync_auto_sync")).length).toBe(1);
  const cloudCall = (await called(page, "sync_auto_sync"))[0];
  expect(cloudCall.args.cloud).toMatchObject({ provider: "groq", model: "whisper-large-v3" });
  await expect(page.getByText("Successfully synced for 1 segments")).toBeVisible();
  await page.getByRole("status").getByRole("button", { name: "Close", exact: true }).click();
  await expect(autoSyncBtn).toBeEnabled();

  // Cancel in-progress Auto-Sync via AutoSyncOverlay
  await page.evaluate(() => window.browserFixture.deferAutoSync(true));
  await autoSyncBtn.click();
  await expect(page.getByRole("heading", { name: "Auto-Sync in progress..." })).toBeVisible();
  await page.evaluate(() => {
    window.browserFixture.emitAutoSyncProgress({
      stage: "transcribe",
      message: "Analyzing segments",
      percentage: 45,
      message_key: null,
      params: null,
    });
  });
  await expect(page.getByText("45%", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect.poll(async () => (await called(page, "sync_cancel_auto_sync")).length).toBe(1);
  await expect(autoSyncBtn).toBeEnabled();

  // Multi-anchor spread display in SyncStatusPanel
  await page.evaluate(() => {
    window.browserFixture.setSyncAnchors([
      { subtitle_id: 1, original_time_ms: 1000, corrected_time_ms: 1200, offset_ms: 200, is_manual: true },
      { subtitle_id: 2, original_time_ms: 5000, corrected_time_ms: 6500, offset_ms: 1500, is_manual: false },
    ]);
    window.browserFixture.selectFile("/exports/sync-session.vestasync");
  });
  await page.getByRole("button", { name: "Load Session" }).click();
  await expect(page.getByText("1.30s", { exact: true })).toBeVisible();
});

test("Extract cancels bitmap & hardcoded ROI OCR and Settings -> ITT manages offline OCR models (EX-03, EX-04, EX-05)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: "Extract subtitles", exact: true }).click();

  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/movie.mkv"));
  await page.getByRole("button", { name: "Browse", exact: true }).first().click();

  // EX-03: Cancel bitmap OCR extraction
  await page.evaluate(() => window.browserFixture.deferOcr(true));
  const trackCard = page.getByRole("group", { name: /English/i });
  await trackCard.getByRole("button", { name: /Save SRT/ }).click();
  await page.evaluate(() => {
    window.browserFixture.emitOcrProgress({ stage: "ocr", message: "Frame 1/4", percentage: 25, current: 1, total: 4 });
  });
  const cancelOcrBtn = page.getByRole("button", { name: "Cancel OCR", exact: true }).first();
  await expect(cancelOcrBtn).toBeVisible();
  await cancelOcrBtn.click();
  await expect.poll(async () => (await called(page, "ocr_cancel")).length).toBe(1);

  // EX-04: Hardcoded ROI frame preview & ROI OCR cancel
  await page.getByRole("tab", { name: "Hardcoded" }).click();
  await expect.poll(async () => (await called(page, "ocr_video_preview_frame")).length).toBeGreaterThan(0);
  await page.evaluate(() => window.browserFixture.deferOcr(true));
  await page.getByRole("button", { name: "Extract ROI to SRT", exact: true }).click();
  await page.evaluate(() => {
    window.browserFixture.emitOcrProgress({ stage: "ocr", message: "ROI 1/2", percentage: 50, current: 1, total: 2 });
  });
  await expect(cancelOcrBtn).toBeVisible();
  await cancelOcrBtn.click();
  await expect.poll(async () => (await called(page, "ocr_cancel")).length).toBe(2);

  // EX-05: Settings -> ITT offline OCR model download & uninstall
  await page.locator("aside > div button").first().click();
  await page.getByRole("button", { name: "ITT" }).click();
  const ocrCard = page.locator("#ocr-models");
  await expect(ocrCard.getByText("0/3", { exact: true })).toBeVisible();
  const latinRadio = ocrCard.getByRole("radio", { name: /PaddleOCR v4 Mobile \(Latin\)/ });
  await latinRadio.getByRole("button", { name: "Download", exact: true }).click();
  await expect.poll(async () => (await called(page, "ocr_download_model")).length).toBe(1);
  await expect(ocrCard.getByText("1/3", { exact: true })).toBeVisible();

  await latinRadio.getByRole("button", { name: "Remove", exact: true }).click({ force: true });
  await expect.poll(async () => (await called(page, "ocr_uninstall_model")).length).toBe(1);
  await expect(ocrCard.getByText("0/3", { exact: true })).toBeVisible();
});

test("Flashcards exports directly to AnkiConnect (RF-03) and Refine cancels AI run with diagnostics (RF-04)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");

  // RF-03: Enable Expert Mode & select AnkiConnect export in Settings -> General
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.locator("aside > div button").first().click();
  await page.getByText("Anki Connect export").click();
  await expect.poll(async () => (await called(page, "ankiconnect_ping")).length).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Upload media" }).click();
  await expect(page.getByRole("cell", { name: "Episode01.ja.srt", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "No folder selected", exact: true }).click();
  await page.evaluate(() => window.browserFixture.setAutoCompleteExport(true));
  await page.getByRole("button", { name: "Generate Flashcards", exact: true }).click();
  await expect.poll(async () => (await called(page, "ankiconnect_import_package")).length).toBe(1);

  // RF-04: Refine deferred batch with failure + Stop cancellation + summary diagnostics
  await page.getByRole("button", { name: /Annotate/ }).click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/deck.apkg"));
  await page.getByRole("button", { name: "Browse flashcard file" }).click();
  await expect.poll(async () => (await called(page, "refine_load_file")).length).toBe(1);

  await page.getByRole("button", { name: "Automatic (AI)" }).click();
  await page.evaluate(() => window.browserFixture.deferRefine(true));
  await page.getByRole("button", { name: "Start AI Refinement" }).click();
  await expect.poll(async () => (await called(page, "refine_cards_llm_tiered")).length).toBe(1);
  const runId = (await called(page, "refine_cards_llm_tiered"))[0]?.args?.runId ?? 1;

  await page.evaluate((id) => {
    window.browserFixture.emitRefineProgress({
      runId: id,
      event: { type: "cardDone", id: "card-1", notes: "Particle note", done: 1, total: 2 },
    });
    window.browserFixture.emitRefineProgress({
      runId: id,
      event: { type: "cardFailed", id: "card-2", error: "Rate limit 429" },
    });
  }, runId);

  await page.getByRole("button", { name: "Stop", exact: true }).click();
  await expect.poll(async () => (await called(page, "refine_cancel")).length).toBe(1);
  await expect(page.getByRole("status").filter({ hasText: /1/ }).first()).toBeVisible();
});

import { test, expect } from "./browser-fixture.mjs";

const called = async (page, command) =>
  page.evaluate(
    (cmd) => window.browserFixture.calls.filter((entry) => entry.command === cmd),
    command,
  );

test("Startup invokes app info and Settings Overview / STT / Anki controls invoke backend IPCs", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");

  await expect.poll(async () => (await called(page, "get_app_info")).length).toBeGreaterThan(0);
  await expect.poll(async () => (await called(page, "get_update_installation")).length).toBeGreaterThan(0);
  await expect(page.getByRole("button", { name: /v0\.34\.0/ })).toBeVisible();

  // NotificationInbox via sidebar version click
  await page.getByRole("button", { name: /v0\.34\.0/ }).click();
  const notifDialog = page.getByRole("dialog", { name: "Notifications" });
  await expect(notifDialog).toBeVisible();
  await notifDialog.getByRole("button", { name: "Close" }).click();
  await expect(notifDialog).toBeHidden();

  // Enable Expert Mode so Anki Templates section is accessible, then test Settings -> General -> STT -> Anki Templates
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.getByRole("button", { name: /Settings/ }).click();

  await page.getByText("Anki Connect export").click();
  await expect.poll(async () => (await called(page, "ankiconnect_ping")).length).toBeGreaterThan(0);

  await page.getByRole("button", { name: "STT" }).click();
  await expect(page.getByText("Whisper model")).toBeVisible();

  const downloadBtn = page.locator("button[title='Download']").first();
  if (await downloadBtn.isVisible().catch(() => false)) {
    await downloadBtn.click();
    await expect.poll(async () => (await called(page, "transcribe_download_model")).length).toBeGreaterThan(0);
  }

  // Open Settings -> Anki Templates -> Manage fonts -> delete & re-download
  await page.getByRole("button", { name: "Anki Templates" }).click();
  await page.getByRole("button", { name: "Manage" }).click();
  const deleteFontBtn = page.getByRole("button", { name: "Delete", exact: true }).first();
  await expect(deleteFontBtn).toBeEnabled();
  await deleteFontBtn.click();
  await expect.poll(async () => (await called(page, "flashcard_delete_font")).length).toBe(1);

  const downloadAllFonts = page.getByRole("button", { name: "Download all fonts" });
  await expect(downloadAllFonts).toBeEnabled();
  await downloadAllFonts.click();
  await expect.poll(async () => (await called(page, "flashcard_download_font")).length).toBe(1);
});

test("ShortcutOverlay toggles with keyboard and ExperimentalTab exercises setup preview & reset", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");

  // Toggle ShortcutOverlay via Shift+F1 (and verify Shift+? alias also closes/opens)
  await page.locator("main").first().click({ position: { x: 10, y: 10 }, force: true });
  await page.keyboard.press("Shift+F1");
  const shortcutsHeading = page.getByRole("heading", { name: "Keyboard shortcuts" });
  await expect(shortcutsHeading).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(shortcutsHeading).toBeHidden();

  await page.keyboard.down("Shift");
  await page.keyboard.press("Slash");
  await page.keyboard.up("Shift");
  await expect(shortcutsHeading).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(shortcutsHeading).toBeHidden();

  // Click sidebar vesta logo 7 times to unlock Experimental tab
  const logoBtn = page.getByRole("button", { name: "vesta", exact: true });
  for (let i = 0; i < 7; i++) {
    await logoBtn.click();
  }

  const expNavBtn = page.getByRole("button", { name: "Experimental" });
  await expect(expNavBtn).toBeVisible();
  await expNavBtn.click();

  await expect(page.getByRole("heading", { name: "First-run setup preview" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Hide Tab" })).toBeVisible();

  // Simulate fresh installation -> ConfirmDialog -> config_reset_installation + reload
  let resetCalls = 0;
  await page.exposeFunction("__onMockResetInstallation", () => {
    resetCalls += 1;
  });
  await page.getByRole("button", { name: "Clear data and restart" }).click();
  const confirmDialog = page.getByRole("dialog");
  await expect(confirmDialog).toBeVisible();
  await confirmDialog.getByRole("button").last().click();
  await expect.poll(() => resetCalls).toBe(1);
});

for (const { locale, width, height } of [
  { locale: "en", width: 1280, height: 800 },
  { locale: "de", width: 520, height: 600 },
  { locale: "ar", width: 460, height: 520 },
]) {
  test(`Settings, Shortcuts and Plugins fit ${width}x${height} in ${locale} without horizontal overflow`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await page.goto(`/browser-tests/harness.html?language=${locale}`);

    const nav = page.locator("aside nav");
    await nav.locator("button").nth(7).click();
    await page.waitForTimeout(100);
    const pluginsOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(pluginsOverflow, `plugins overflow in ${locale}`).toBeLessThanOrEqual(1);

    await page.locator("aside > div button").first().click();
    await page.waitForTimeout(100);
    const settingsOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(settingsOverflow, `settings overflow in ${locale}`).toBeLessThanOrEqual(1);

    const sectionButtons = page.locator("main button");
    await sectionButtons.nth(6).click();
    await page.waitForTimeout(100);
    const shortcutsOverflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(shortcutsOverflow, `shortcuts overflow in ${locale}`).toBeLessThanOrEqual(1);
  });
}

test("manages Whisper/VAD models (TR-04), Anki font & lookup preferences (FC-07), API keys (GS-02), and Support logs + Kill Switch (GS-06)", async ({ page }) => {
  await page.goto("/browser-tests/harness.html");

  // Enable Expert Mode so Anki Templates section is accessible
  await page.getByRole("switch", { name: "Expert Mode" }).click();
  await page.locator("aside > div button").first().click();

  // TR-04: Settings -> STT model & Silero VAD download/uninstall lifecycle
  await page.getByRole("button", { name: "STT" }).click();
  await page.locator("#tiny button[title='Download']").click();
  await expect.poll(async () => (await called(page, "transcribe_download_model")).length).toBeGreaterThan(0);
  await expect(page.locator("#tiny")).toContainText(/Default|Ready/);
  await page.locator("#tiny button").click({ force: true });
  await expect.poll(async () => (await called(page, "transcribe_uninstall_model")).length).toBeGreaterThan(0);

  const vadSection = page.locator("#silero-vad");
  await vadSection.getByRole("button", { name: "Uninstall", exact: true }).click();
  await expect.poll(async () => (await called(page, "transcribe_uninstall_vad")).length).toBe(1);
  await vadSection.getByRole("button", { name: "Download", exact: true }).click();
  await expect.poll(async () => (await called(page, "transcribe_download_vad")).length).toBe(1);

  // GS-02: Settings -> LLM API keys add, edit, and delete
  await page.getByRole("button", { name: "LLM" }).click();
  await page.getByRole("button", { name: "Add configuration", exact: true }).click();
  const keyDialog = page.getByRole("dialog");
  await keyDialog.getByRole("button", { name: /^OpenAI\b/i }).click();
  await keyDialog.locator("#key-name").fill("E2E OpenAI");
  await keyDialog.locator("#api-key").fill("sk-e2e-test-key");
  await keyDialog.getByRole("button", { name: "Save", exact: true }).click();
  await expect(keyDialog).toBeHidden();
  await expect(page.getByText("E2E OpenAI", { exact: true })).toBeVisible();

  await page.getByText("E2E OpenAI", { exact: true }).click();
  await keyDialog.locator("#key-name").fill("E2E OpenAI Edited");
  await keyDialog.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("E2E OpenAI Edited", { exact: true })).toBeVisible();

  await page.getByText("E2E OpenAI Edited", { exact: true }).click();
  await keyDialog.locator("button[data-provider-action='delete']").click();
  const deleteConfirm = page.getByRole("dialog", { name: "Delete provider" });
  await deleteConfirm.getByRole("button").last().click();
  await expect(page.getByText("E2E OpenAI Edited", { exact: true })).toHaveCount(0);
  const apiKeyWrites = (await called(page, "config_set")).filter((c) => c.args?.key === "vesta-api-keys");
  expect(apiKeyWrites.length).toBeGreaterThanOrEqual(3);

  // FC-07: Settings -> Anki Templates toggles propagate to flashcard_generate
  await page.getByRole("button", { name: "Anki Templates" }).click();
  const lookupSwitch = page.getByRole("switch", { name: "Word lookup" });
  const autoFontSwitch = page.getByRole("switch", { name: "Automatic font" });
  const embedFontSwitch = page.getByRole("switch", { name: "Include font in deck" });
  await lookupSwitch.click();
  await autoFontSwitch.click();
  await expect(embedFontSwitch).toBeDisabled();
  await autoFontSwitch.click();
  await expect(embedFontSwitch).toBeEnabled();

  await page.getByRole("button", { name: "Manage", exact: true }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).first().click();
  await expect.poll(async () => (await called(page, "flashcard_delete_font")).length).toBe(1);
  await page.getByRole("button", { name: "Download", exact: true }).click();
  await expect.poll(async () => (await called(page, "flashcard_download_font")).length).toBe(1);

  // GS-06 (part 1): Settings -> Diagnostics support log record, export, stop
  await page.getByRole("button", { name: "Diagnostics" }).click();
  await page.getByRole("button", { name: "Record", exact: true }).click();
  await expect.poll(async () => (await called(page, "support_log_start")).length).toBe(1);
  await page.evaluate(() => window.browserFixture.selectFile("/exports/e2e-support.jsonl"));
  await page.getByRole("button", { name: "Export logs", exact: true }).click();
  await expect.poll(async () => (await called(page, "support_log_export")).length).toBe(1);
  const toastClose = page.getByRole("status").getByRole("button", { name: "Close", exact: true });
  await toastClose.waitFor({ state: "visible", timeout: 2000 }).catch(() => {});
  if (await toastClose.isVisible()) await toastClose.click();
  await page.getByRole("button", { name: "Stop recording", exact: true }).click();
  await expect.poll(async () => (await called(page, "support_log_stop")).length).toBe(1);

  // Exit Settings -> Flashcards verify FC-07 preferences flow into flashcard_generate
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Upload media" }).click();
  await expect(page.getByRole("cell", { name: "Episode01.ja.srt", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "No folder selected", exact: true }).click();
  await page.evaluate(() => window.browserFixture.setAutoCompleteExport(true));
  await page.getByRole("button", { name: "Generate Flashcards", exact: true }).click();
  await expect.poll(async () => (await called(page, "flashcard_generate")).length).toBe(1);
  const genCall = (await called(page, "flashcard_generate"))[0];
  expect(genCall.args.config).toMatchObject({
    dictionary_lookup: false,
    auto_card_font: true,
    embed_card_font: true,
  });

  // GS-06 (part 2): AI Kill Switch cancels an in-flight translation
  await page.getByRole("button", { name: "Translate", exact: true }).click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/Episode01.ja.srt"));
  await page.getByRole("button", { name: "Select input SRT file" }).click();
  await page.getByRole("button", { name: "Start Translation" }).click();
  await expect.poll(async () => (await called(page, "start_translation")).length).toBe(1);
  await page.getByRole("switch", { name: "AI Kill Switch", exact: true }).click();
  await expect.poll(async () => (await called(page, "cancel_translation")).length).toBe(1);
});

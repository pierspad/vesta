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

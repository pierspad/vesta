import { readFileSync } from "node:fs";
import { expect, test } from "./browser-fixture.mjs";

const scenarios = [
  { language: "en", width: 1440, height: 1000 },
  { language: "de", width: 520, height: 600 },
  { language: "ar", width: 460, height: 520 },
];

for (const { language, width, height } of scenarios) {
  test(`primary tabs fit ${width}×${height} in ${language}`, async ({ page }, testInfo) => {
    const messages = JSON.parse(readFileSync(
      new URL(`../src/lib/i18n/locales/${language}.json`, import.meta.url), "utf8",
    ));
    await page.setViewportSize({ width, height });
    await page.goto(`/browser-tests/harness.html?language=${language}`);
    await expect(page.locator("html")).toHaveAttribute("lang", language);
    await expect(page.locator("html")).toHaveAttribute("dir", language === "ar" ? "rtl" : "ltr");

    for (const tab of ["flashcards", "sync", "revision", "refine", "translate", "transcribe", "extract"]) {
      await test.step(tab, async () => {
        await page.getByRole("navigation").getByRole("button", { name: messages[`nav.${tab}`], exact: true }).click();
        const footer = page.locator("[data-footer-actions]:visible");
        await expect(footer).toBeVisible();
        const viewport = await page.evaluate(() => ({
          width: document.documentElement.clientWidth,
          content: document.documentElement.scrollWidth,
        }));
        expect(viewport.content).toBeLessThanOrEqual(viewport.width);

        const collisions = await footer.getByRole("button").evaluateAll((buttons) => {
          const bounds = buttons.map(button => button.getBoundingClientRect());
          const band = buttons[0]?.closest("[data-footer-actions]")?.getBoundingClientRect();
          return bounds.flatMap((a, i) => {
            const label = buttons[i].textContent.trim();
            const outside = a.left < Math.max(0, band.left) - 1 || a.right > Math.min(innerWidth, band.right) + 1 ||
              a.top < Math.max(0, band.top) - 1 || a.bottom > Math.min(innerHeight, band.bottom) + 1;
            const overlaps = bounds.slice(i + 1).some((b, j) =>
              !buttons[i].contains(buttons[i + j + 1]) &&
              Math.min(a.right, b.right) - Math.max(a.left, b.left) > 2 &&
              Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 2);
            return outside || overlaps ? [label] : [];
          });
        });
        expect(collisions).toEqual([]);
        if (tab === "flashcards" || tab === "sync") {
          await page.mouse.move(1, 1);
          await page.screenshot({ path: testInfo.outputPath(`${tab}.png`), animations: "disabled" });
        }
      });
    }
  });
}

test("Revise and Extract pin empty-state labels and toolbars while scrolling content", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/browser-tests/harness.html?language=en");

  // Revise (AlignTab)
  await page.getByRole("navigation").getByRole("button", { name: "Revise", exact: true }).click();
  await expect(page.getByText("Waiting for the translation file...")).toHaveCount(0);
  await expect(page.getByText("No Subtitle", { exact: true })).toHaveCount(30);

  const alignEmpty = page.locator("span:visible", { hasText: /^No subtitles loaded$/ });
  await expect(alignEmpty).toBeVisible();
  const alignLabelBefore = await alignEmpty.boundingBox();
  expect(alignLabelBefore).not.toBeNull();
  expect(alignLabelBefore.y).toBeGreaterThan(100);
  expect(alignLabelBefore.y + alignLabelBefore.height).toBeLessThan(800);

  const nextBtn = page.getByRole("button", { name: /Next/i }).first();
  const nextBefore = await nextBtn.boundingBox();

  const alignScroller = page.locator("div.overflow-y-auto.custom-scrollbar:visible").first();
  const alignCanScroll = await alignScroller.evaluate((el) => {
    const before = el.scrollTop;
    el.scrollTop = 300;
    return el.scrollHeight > el.clientHeight && el.scrollTop > before;
  });
  expect(alignCanScroll).toBe(true);

  const alignLabelAfter = await alignEmpty.boundingBox();
  const nextAfter = await nextBtn.boundingBox();
  expect(Math.abs(alignLabelAfter.y - alignLabelBefore.y)).toBeLessThanOrEqual(1);
  expect(Math.abs(nextAfter.y - nextBefore.y)).toBeLessThanOrEqual(1);

  // Extract subtitles (ExtractTab)
  await page.getByRole("navigation").getByRole("button", { name: "Extract subtitles", exact: true }).click();
  const extractEmpty = page.locator("span:visible", { hasText: /^No media loaded$/ });
  await expect(extractEmpty).toBeVisible();
  const extractLabelBefore = await extractEmpty.boundingBox();
  expect(extractLabelBefore).not.toBeNull();
  expect(extractLabelBefore.y + extractLabelBefore.height).toBeLessThan(800);

  const searchInput = page.getByRole("searchbox", { name: /Search language, title or format/ });
  const searchBefore = await searchInput.boundingBox();

  const extractScroller = page.locator("div.overflow-y-auto.custom-scrollbar:visible").first();
  const extractCanScroll = await extractScroller.evaluate((el) => {
    const before = el.scrollTop;
    el.scrollTop = 200;
    return el.scrollHeight > el.clientHeight && el.scrollTop > before;
  });
  expect(extractCanScroll).toBe(true);

  const extractLabelAfter = await extractEmpty.boundingBox();
  const searchAfter = await searchInput.boundingBox();
  expect(Math.abs(extractLabelAfter.y - extractLabelBefore.y)).toBeLessThanOrEqual(1);
  expect(Math.abs(searchAfter.y - searchBefore.y)).toBeLessThanOrEqual(1);
});

const ALL_LOCALES = ["ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "nl", "pl", "pt", "ru", "tr", "zh"];

test("all 15 locales render sidebar navigation and document direction without raw translation keys (QC-03)", async ({ page }) => {
  for (const locale of ALL_LOCALES) {
    await test.step(locale, async () => {
      const messages = JSON.parse(readFileSync(
        new URL(`../src/lib/i18n/locales/${locale}.json`, import.meta.url), "utf8",
      ));
      await page.goto(`/browser-tests/harness.html?language=${locale}`);
      await expect(page.locator("html")).toHaveAttribute("lang", locale);
      await expect(page.locator("html")).toHaveAttribute("dir", locale === "ar" ? "rtl" : "ltr");
      await expect(page.getByRole("navigation").getByRole("button", { name: messages["nav.flashcards"], exact: true })).toBeVisible();

      const navButtons = page.getByRole("navigation").getByRole("button");
      const texts = (await navButtons.allTextContents()).map((s) => s.trim()).filter(Boolean);
      expect(texts.length).toBeGreaterThanOrEqual(7);
      for (const label of texts) {
        expect(label).not.toMatch(/^nav\./);
      }
    });
  }
});

test("Italian Sync keeps Auto-Sync controls on one row and stretches both panels vertically at 1280x800", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/browser-tests/harness.html?language=it");
  await page.getByRole("navigation").getByRole("button", { name: "Sincronizza", exact: true }).click();

  const footer = page.locator("[data-footer-actions]:visible");
  const autoSyncBtn = footer.getByRole("button", { name: "Sincronizzazione automatica", exact: true });
  const fastBtn = footer.getByRole("button", { name: /Veloce/i });
  const preciseBtn = footer.getByRole("button", { name: /Accurato/i });
  await expect(autoSyncBtn).toBeVisible();
  await expect(fastBtn).toBeVisible();

  const autoBox = await autoSyncBtn.boundingBox();
  const fastBox = await fastBtn.boundingBox();
  const preciseBox = await preciseBtn.boundingBox();
  expect(autoBox).not.toBeNull();
  expect(fastBox).not.toBeNull();
  expect(preciseBox).not.toBeNull();
  // Action button and mode switcher share the same horizontal band on desktop
  expect(Math.abs((autoBox.y + autoBox.height / 2) - (fastBox.y + fastBox.height / 2))).toBeLessThanOrEqual(6);
  expect(fastBox.x).toBeGreaterThan(autoBox.x + autoBox.width - 1);
  expect(preciseBox.x).toBeGreaterThan(fastBox.x + fastBox.width - 2);

  // Subtitle panel stretches near the footer rather than stopping prematurely
  const footerBox = await footer.boundingBox();
  const listHeading = page.getByRole("heading", { name: /Sottotitoli/ });
  const listCardBox = await listHeading.locator("xpath=ancestor::div[contains(@class,'glass-card')][1]").boundingBox();
  expect(footerBox).not.toBeNull();
  expect(listCardBox).not.toBeNull();
  expect(footerBox.y - (listCardBox.y + listCardBox.height)).toBeLessThanOrEqual(48);
});

test("Extract hardcoded mode centers seek and area proportion controls without ROI jargon", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/browser-tests/harness.html?language=en");
  await page.getByRole("navigation").getByRole("button", { name: "Extract subtitles", exact: true }).click();
  await page.evaluate(() => window.browserFixture.selectFile("/fixtures/movie.mkv"));
  await page.getByRole("button", { name: "Browse", exact: true }).first().click();
  await page.getByRole("tab", { name: "Hardcoded", exact: true }).click();

  const panel = page.locator("#hardcoded-roi-panel");
  await expect(panel.getByRole("heading", { name: "Hardcoded subtitles", exact: true })).toBeVisible();
  await expect(panel).not.toContainText(/\bROI\b/);

  const xInputBox = await panel.getByLabel("X (%)").boundingBox();
  const hInputBox = await panel.getByLabel("H (%)").boundingBox();
  const seekBtnBox = await panel.getByRole("button", { name: "−30s", exact: true }).boundingBox();
  const seekPlusBox = await panel.getByRole("button", { name: "+30s", exact: true }).boundingBox();
  expect(xInputBox && hInputBox && seekBtnBox && seekPlusBox).not.toBeNull();
  // Dimensions bar sits cleanly in its own row below the centered seek row
  expect(xInputBox.y).toBeGreaterThan(seekBtnBox.y + seekBtnBox.height);
  const seekCenter = (seekBtnBox.x + seekPlusBox.x + seekPlusBox.width) / 2;
  const dimsCenter = (xInputBox.x + hInputBox.x + hInputBox.width) / 2;
  expect(Math.abs(seekCenter - dimsCenter)).toBeLessThanOrEqual(40);

  // 16:9 video (1920x1080) inside the fixed 21:9 preview stage keeps 16:9 aspect with side pillarbox bars
  const stageBox = await panel.getByTestId("video-preview-stage").boundingBox();
  const frameBox = await panel.getByTestId("roi-frame-stage").boundingBox();
  expect(stageBox && frameBox).not.toBeNull();
  expect(Math.abs(stageBox.width / stageBox.height - 21 / 9)).toBeLessThan(0.05);
  expect(Math.abs(frameBox.width / frameBox.height - 16 / 9)).toBeLessThan(0.05);
  expect(frameBox.width).toBeLessThan(stageBox.width - 100);
});

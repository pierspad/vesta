import { expect, test } from "./browser-fixture.mjs";

const callsFor = (page, command) => page.evaluate(
  (name) => window.browserFixture.calls.filter((call) => call.command === name),
  command,
);

const config = {
  target_subs_path: "/fixtures/Episode01.ja.srt",
  video_path: "/fixtures/Episode01.mp4",
  output_dir: "/exports",
  export_format: "apkg",
};

const previewButton = (page) => page.getByRole("button", { name: "Preview Cards", exact: true });
const generateButton = (page) => page.getByRole("button", { name: "Generate Flashcards", exact: true });
const outputAction = (page) => page.getByRole("button", { name: "Open with default application: /exports/Episode01.apkg" });

test.beforeEach(async ({ page }) => {
  await page.goto("/browser-tests/harness.html");
  await page.getByRole("button", { name: "Upload media" }).click();
  await expect(page.getByRole("cell", { name: "Episode01.ja.srt", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Episode01.mp4", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "No folder selected", exact: true }).click();
  await expect(page.getByRole("button", { name: "/exports", exact: true })).toBeVisible();
});

test("previews selected files and exports flashcards", async ({ page }, testInfo) => {
  await previewButton(page).click();
  const dialog = page.getByRole("dialog", { name: "Preview Cards" });
  await expect(dialog).toBeFocused();
  await expect(page.getByRole("tooltip")).toBeHidden();
  await expect(dialog.getByText("Synthetic subtitle for the browser test", { exact: true })).toBeVisible();
  expect(await callsFor(page, "flashcard_preview")).toMatchObject([{ args: { config } }]);
  await page.mouse.move(0, 0);
  await page.screenshot({ path: testInfo.outputPath("preview.png"), animations: "disabled" });
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(previewButton(page)).toBeFocused();

  await generateButton(page).click();
  await expect.poll(() => callsFor(page, "flashcard_generate")).toMatchObject([{
    args: { config: { ...config, target_subs_path: "/fixtures/preview-target.srt" } },
  }]);
  expect(await callsFor(page, "save_temp_subtitles")).toMatchObject([{
    args: { useNative: false, lines: [{ subs1_text: "Synthetic subtitle for the browser test" }] },
  }]);
  await page.evaluate(() => window.browserFixture.progress());
  await expect(page.getByText("40%", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Cancel", exact: true })).toBeVisible();

  await page.evaluate(() => window.browserFixture.complete());
  await expect(page.getByText("1 cards generated", { exact: true })).toBeVisible();
  await expect(outputAction(page)).toBeVisible();
  await page.mouse.move(0, 0);
  await page.screenshot({ path: testInfo.outputPath("export.png"), animations: "disabled" });
});

test("shows an unreadable subtitle error and can retry the preview", async ({ page }) => {
  await page.evaluate(() => window.browserFixture.failPreview("Cannot read subtitle fixture"));
  await previewButton(page).click();
  await expect(page.getByRole("alert").filter({ hasText: "Cannot read subtitle fixture" })).toBeVisible();
  await expect(page.getByRole("dialog")).toBeHidden();

  await page.evaluate(() => window.browserFixture.failPreview(null));
  await previewButton(page).click();
  await expect(page.getByRole("dialog").getByText("Synthetic subtitle for the browser test")).toBeVisible();
});

test("cancels a running export without showing a package", async ({ page }) => {
  await generateButton(page).click();
  await expect.poll(() => callsFor(page, "flashcard_generate")).toHaveLength(1);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();

  await expect.poll(() => callsFor(page, "flashcard_cancel")).toHaveLength(1);
  await expect(page.getByText(/Cancelled by user/)).toBeVisible();
  await expect(page.getByRole("button", { name: "New Generation", exact: true })).toBeVisible();
  await expect(outputAction(page)).toHaveCount(0);
});

test("exports bilingual dueling subtitles from preview", async ({ page }) => {
  await page.evaluate(() => window.browserFixture.selectFile(["/fixtures/Episode01.en.srt"]));
  await page.getByRole("button", { name: "Upload media" }).click();
  await expect(page.getByRole("cell", { name: "Episode01.en.srt", exact: true })).toBeVisible();

  await previewButton(page).click();
  const dialog = page.getByRole("dialog", { name: "Preview Cards" });
  await expect(dialog.getByText("Reference subtitle", { exact: true })).toBeVisible();

  await page.evaluate(() => window.browserFixture.selectFile("/exports/Episode01.dueling.ass"));
  await dialog.getByRole("button", { name: "Export Dueling", exact: true }).click();

  await expect.poll(() => callsFor(page, "flashcard_export_dueling")).toMatchObject([{
    args: {
      config: {
        target_subs_path: "/fixtures/preview-target.srt",
        native_subs_path: "/fixtures/preview-native.srt",
      },
      options: {
        format: "ass",
        output_path: "/exports/Episode01.dueling.ass",
        target_first: true,
        include_unmatched: true,
      },
    },
  }]);
  await expect(page.getByText("Exported 1 bilingual lines to Episode01.dueling.ass")).toBeVisible();
});

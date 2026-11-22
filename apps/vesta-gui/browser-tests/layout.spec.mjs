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

import { test, expect } from "./browser-fixture.mjs";

for (const viewport of [{ width: 1280, height: 900 }, { width: 480, height: 520 }]) {
  test(`Notifications keep setup and installed release notes accessible at ${viewport.width}px`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto('/browser-tests/harness.html');
    const opener = page.getByRole('button', { name: 'Notifications', exact: true });
    await expect(opener).toBeVisible();
    await page.evaluate(async () => {
      const { updateCheckerStore: updates } = await import('/src/lib/stores/updateCheckerStore.svelte.ts');
      await updates.loadInstalledReleaseNotes();
      updates.loadInstalledReleaseNotes = async () => {};
      updates.updateStatus = 'available';
      updates.latestVersion = 'v1.4.0';
      updates.appVersionNum = 'v1.3.0';
      updates.installedReleaseNotes = '## Installed changes\n\n' + '- Improved subtitle workflows.\n'.repeat(30);
      window.dispatchEvent(new CustomEvent('vesta-settings-action-required-changed', { detail: {
        required: true, hash: 'whisper-model-missing:v1|llm-default-unready:v1|itt-model-missing:v1', hidden: false, read: false,
      } }));
    });
    await opener.click();
    const dialog = page.getByRole('dialog', { name: 'Notifications', exact: true });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText('Optional setup', { exact: true })).toBeVisible();
    await expect(dialog.locator('.markdown-content')).toHaveCount(0);
    await expect(dialog.getByText('New version available: v1.4.0', { exact: true })).toBeVisible();
    if (process.env.VESTA_UI_REVIEW_DIR) await page.screenshot({ path: `${process.env.VESTA_UI_REVIEW_DIR}/notifications-${viewport.width}.png` });
    const details = dialog.getByRole('button', { name: 'Installed version v1.3.0' });
    await details.click();
    await expect(details).toHaveAttribute('aria-expanded', 'true');
    await expect(dialog.getByRole('heading', { name: 'Installed changes' })).toBeVisible();
    const link = dialog.locator('.release-link');
    await link.scrollIntoViewIfNeeded();
    await expect(link).toBeVisible();
    const box = await dialog.boundingBox();
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.y + box.height).toBeLessThanOrEqual(viewport.height);
    const body = dialog.locator('.notification-body');
    expect(await body.evaluate(node => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    await details.click();
    await expect(dialog.locator('.markdown-content')).toHaveCount(0);
    await dialog.getByRole('button', { name: 'Close', exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(opener).toBeFocused();
  });
}


test("Plugin handoff buttons share workspace colors and their icons inherit the color", async ({ page }) => {
  await page.goto('/browser-tests/harness.html');
  await expect(page.getByRole('button', { name: 'Plugins', exact: true })).toBeVisible();
  await page.evaluate(() => window.browserFixture.setPluginView({ root: { type: 'column', children: [
    { type: 'heading', text: 'Downloads', level: 2 },
    { type: 'card', title: 'Episode 01', children: [
      { type: 'fileOutput', label: 'Saved subtitle', path: '/downloads/Episode01.en.srt' },
      { type: 'row', children: [
        { type: 'button', id: 'flashcards', label: 'Use in Flashcards', variant: 'flashcards', icon: 'cards' },
        { type: 'button', id: 'synchronize', label: 'Use in Synchronize', variant: 'synchronize', icon: 'refresh' },
      ] },
    ] },
  ] } }));
  await page.getByRole('button', { name: 'Plugins', exact: true }).click();
  await page.getByPlaceholder('https://github.com/owner/repository').fill('https://github.com/vesta/demo-plugin');
  await page.getByRole('button', { name: 'Install Plugin', exact: true }).click();
  await page.locator('dialog').getByRole('button', { name: 'Confirm & Install', exact: true }).click();
  await page.getByRole('button', { name: 'Open', exact: true }).click();
  for (const [id, color] of [['flashcards', '--workflow-flashcards'], ['synchronize', '--workflow-synchronize']]) {
    const button = page.locator(`[data-plugin-button="${id}"]`);
    await expect(button).toBeVisible();
    const colors = await button.evaluate((node, token) => {
      const reference = document.createElement('span');
      reference.style.color = `var(${token})`;
      node.append(reference);
      const expected = getComputedStyle(reference).color;
      reference.remove();
      return { button: getComputedStyle(node).color, icon: getComputedStyle(node.querySelector('svg')).color, expected };
    }, color);
    expect(colors.button).toBe(colors.expected);
    expect(colors.icon).toBe(colors.button);
  }
  if (process.env.VESTA_UI_REVIEW_DIR) await page.screenshot({ path: `${process.env.VESTA_UI_REVIEW_DIR}/plugin-workflows.png` });
});

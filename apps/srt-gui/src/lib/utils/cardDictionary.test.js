// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
const script = readFileSync(process.cwd() + "/../../lib/srt-flashcards/src/export_apkg/dictionary.js", "utf8");
function run(language = "ja") {
  new Function(script.replace("__VESTA_LANGUAGE__", JSON.stringify(language)))();
}
afterEach(() => { document.body.innerHTML = ""; vi.unstubAllGlobals(); });
it("segments Japanese, preserves ruby and existing links, and is idempotent", () => {
  document.body.innerHTML = '<div class="expression"><ruby>日本語<rt>にほんご</rt></ruby>を勉強する。<a href="https://example.org">existing</a></div>';
  const original = document.querySelector('.expression').textContent;
  run();
  const links = document.querySelectorAll('.expression a[href*="wiktionary"]');
  expect(links.length).toBeGreaterThan(1);
  expect(links[0].getAttribute('href')).toContain('#Japanese');
  expect(document.querySelector('rt a')).toBeNull();
  expect(document.querySelector('a[href="https://example.org"]').textContent).toBe('existing');
  expect(document.querySelector('.expression').textContent).toBe(original);
  run();
  expect(document.querySelectorAll('.expression a[href*="wiktionary"]')).toHaveLength(links.length);
});
it("supports selection and prevents empty lookups on older webviews", () => {
  vi.stubGlobal('Intl', {});
  document.body.innerHTML = '<div class="expression">hello world</div>';
  run('en');
  const link = document.querySelector('a');
  expect(link.dispatchEvent(new MouseEvent('click', {cancelable: true}))).toBe(false);
  const range = document.createRange();
  range.selectNodeContents(document.querySelector('.expression'));
  window.getSelection().addRange(range);
  link.addEventListener('click', event => event.preventDefault());
  link.dispatchEvent(new MouseEvent('click', {cancelable: true}));
  expect(link.href).toBe('https://en.wiktionary.org/wiki/hello%20world#English');
});

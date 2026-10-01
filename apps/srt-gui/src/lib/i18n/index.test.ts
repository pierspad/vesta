import { describe, it, expect } from 'vitest';
import { t, availableUILanguages } from './index';

describe('i18n', () => {
  it('has valid available UI languages with required properties', () => {
    expect(availableUILanguages.length).toBeGreaterThanOrEqual(10);
    for (const lang of availableUILanguages) {
      expect(lang.code).toBeTruthy();
      expect(lang.name).toBeTruthy();
      expect(lang.nativeName).toBeTruthy();
      expect(lang.flag).toBeTruthy();
    }
  });

  it('translates common keys into english default', () => {
    const cancel = t('common.cancel');
    expect(cancel).toBe('Cancel');

    const appName = t('app.title');
    expect(appName).toBe('vesta');
  });

  it('interpolates parameters correctly', () => {
    const interpolated = t('align.subtitlesLoaded', { count: 5 });
    expect(interpolated).toBe('5 subtitles loaded');
  });

  it('falls back to key itself when translation is missing in all dictionaries', () => {
    const missingKey = ['non', 'existent', 'key', '12345'].join('.');
    const missing = t(missingKey);
    expect(missing).toBe(missingKey);
  });
});

it('translates a wizard language without changing the active application language', async () => {
  const { loadLanguage, translateForLanguage, getLanguage } = await import('./index');
  const active = getLanguage();
  expect(await loadLanguage('de')).toBe(true);
  expect(translateForLanguage('de', 'common.cancel')).toBe('Abbrechen');
  expect(getLanguage()).toBe(active);
});

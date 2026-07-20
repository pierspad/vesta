/**
 * Sistema di internazionalizzazione (i18n)
 *
 * Struttura semplice e manutenibile:
 * - Un file JSON per ogni lingua in /locales/
 * - Riferimenti generici nel codice tramite chiavi
 * - Facile aggiungere nuove lingue
 *
 * Solo `en` (fallback) è nel bundle principale; le altre lingue sono
 * chunk separati caricati on-demand da `loadLanguage` — così il main
 * chunk non trasporta ~800KB di traduzioni mai usate.
 */

import { derived, get, writable } from 'svelte/store';
import en from '$lib/i18n/locales/en.json';
import * as vestaConfig from "$lib/config/vestaConfig";

export interface UILanguage {
  code: string;
  name: string;
  nativeName: string;
  flag: string;
}

export const availableUILanguages: UILanguage[] = [
  { code: 'ar', name: 'Arabic', nativeName: 'العربية', flag: '🇸🇦' },
  { code: 'zh', name: 'Chinese', nativeName: '中文', flag: '🇨🇳' },
  { code: 'nl', name: 'Dutch', nativeName: 'Nederlands', flag: '🇳🇱' },
  { code: 'en', name: 'English', nativeName: 'English', flag: '🇬🇧' },
  { code: 'fr', name: 'French', nativeName: 'Français', flag: '🇫🇷' },
  { code: 'de', name: 'German', nativeName: 'Deutsch', flag: '🇩🇪' },
  { code: 'hi', name: 'Hindi', nativeName: 'हिन्दी', flag: '🇮🇳' },
  { code: 'it', name: 'Italian', nativeName: 'Italiano', flag: '🇮🇹' },
  { code: 'ja', name: 'Japanese', nativeName: '日本語', flag: '🇯🇵' },
  { code: 'ko', name: 'Korean', nativeName: '한국어', flag: '🇰🇷' },
  { code: 'pl', name: 'Polish', nativeName: 'Polski', flag: '🇵🇱' },
  { code: 'pt', name: 'Portuguese', nativeName: 'Português', flag: '🇵🇹' },
  { code: 'ru', name: 'Russian', nativeName: 'Русский', flag: '🇷🇺' },
  { code: 'es', name: 'Spanish', nativeName: 'Español', flag: '🇪🇸' },
  { code: 'tr', name: 'Turkish', nativeName: 'Türkçe', flag: '🇹🇷' },
];

const supportedCodes = new Set(availableUILanguages.map((l) => l.code));

const translations: Record<string, Record<string, string>> = {
  en: { ...en },
};

/** Bumped whenever a locale finishes loading, so `locale` re-derives. */
const revision = writable(0);

const localeModules = import.meta.glob<Record<string, string>>('./locales/*.json', {
  import: 'default',
});

/**
 * Load (once) the dictionary for `lang` into `translations`.
 * Resolves to true when the language is available afterwards.
 */
export async function loadLanguage(lang: string): Promise<boolean> {
  if (translations[lang]) return true;
  const loader = localeModules[`./locales/${lang}.json`];
  if (!loader) return false;
  try {
    const dict = await loader();
    translations[lang] = { ...en, ...dict };
    revision.update((n) => n + 1);
    return true;
  } catch (e) {
    console.error(`[i18n] failed to load locale "${lang}"`, e);
    return false;
  }
}

const STORAGE_KEY = 'srt-tools-ui-language';

function getSystemLanguage(): string {
  if (typeof navigator !== 'undefined') {
    // Prova prima navigator.language (es: "it-IT", "en-US")
    const fullLang = navigator.language;
    const shortLang = fullLang.split('-')[0].toLowerCase();

    if (supportedCodes.has(shortLang)) {
      return shortLang;
    }

    // Prova navigator.languages per lingue alternative
    if (navigator.languages) {
      for (const lang of navigator.languages) {
        const short = lang.split('-')[0].toLowerCase();
        if (supportedCodes.has(short)) {
          return short;
        }
      }
    }
  }
  return 'en';
}

function getInitialLanguage(): string {
  const saved = vestaConfig.getItem(STORAGE_KEY);
  if (saved && supportedCodes.has(saved)) {
    return saved;
  }
  // Usa la lingua del sistema operativo come default
  return getSystemLanguage();
}

export const currentLanguage = writable<string>(getInitialLanguage());

currentLanguage.subscribe((lang) => {
  vestaConfig.setItem(STORAGE_KEY, lang);
});

/**
 * Ensure the dictionary for the initial language is loaded.
 * main.ts awaits this before mounting App so the first paint is already
 * in the right language.
 */
export async function initI18n(): Promise<void> {
  await loadLanguage(get(currentLanguage));
}

function translate(
  lang: string,
  key: string,
  params?: Record<string, string | number>
): string {
  const translation = translations[lang]?.[key] || translations['en']?.[key] || key;

  if (params) {
    return Object.entries(params).reduce(
      (str, [k, v]) => str.replace(new RegExp(`{{${k}}}`, 'g'), String(v)),
      translation
    );
  }

  return translation;
}

export function t(key: string, params?: Record<string, string | number>): string {
  return translate(get(currentLanguage), key, params);
}

export const locale = derived([currentLanguage, revision], ([$lang]) => {
  return (key: string, params?: Record<string, string | number>): string => {
    return translate($lang, key, params);
  };
});

export function setLanguage(lang: string): void {
  if (!supportedCodes.has(lang)) return;
  void loadLanguage(lang).then((ok) => {
    if (ok) currentLanguage.set(lang);
  });
}

export function getLanguage(): string {
  return get(currentLanguage);
}

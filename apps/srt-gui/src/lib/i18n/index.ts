/**
 * Sistema di internazionalizzazione (i18n)
 * 
 * Struttura semplice e manutenibile:
 * - Un file JSON per ogni lingua in /locales/
 * - Riferimenti generici nel codice tramite chiavi
 * - Facile aggiungere nuove lingue
 */

import { derived, get, writable } from 'svelte/store';
import ar from './locales/ar.json';
import de from './locales/de.json';
import en from './locales/en.json';
import es from './locales/es.json';
import fr from './locales/fr.json';
import hi from './locales/hi.json';
import it from './locales/it.json';
import ja from './locales/ja.json';
import ko from './locales/ko.json';
import pl from './locales/pl.json';
import pt from './locales/pt.json';
import ru from './locales/ru.json';
import tr from './locales/tr.json';
import zh from './locales/zh.json';

// Info/help content — verbose HTML kept in a parallel folder for cleanliness
import infoAr from './locales/info/ar.json';
import infoDe from './locales/info/de.json';
import infoEn from './locales/info/en.json';
import infoEs from './locales/info/es.json';
import infoFr from './locales/info/fr.json';
import infoHi from './locales/info/hi.json';
import infoIt from './locales/info/it.json';
import infoJa from './locales/info/ja.json';
import infoKo from './locales/info/ko.json';
import infoPl from './locales/info/pl.json';
import infoPt from './locales/info/pt.json';
import infoRu from './locales/info/ru.json';
import infoTr from './locales/info/tr.json';
import infoZh from './locales/info/zh.json';

export interface UILanguage {
  code: string;
  name: string;
  nativeName: string;
  flag: string;
}

export const availableUILanguages: UILanguage[] = [
  { code: 'ar', name: 'Arabic', nativeName: 'العربية', flag: '🇸🇦' },
  { code: 'zh', name: 'Chinese', nativeName: '中文', flag: '🇨🇳' },
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

const translations: Record<string, Record<string, string>> = {
  ar: { ...en, ...infoEn, ...ar, ...infoAr },
  de: { ...en, ...infoEn, ...de, ...infoDe },
  en: { ...en, ...infoEn },
  es: { ...en, ...infoEn, ...es, ...infoEs },
  fr: { ...en, ...infoEn, ...fr, ...infoFr },
  hi: { ...en, ...infoEn, ...hi, ...infoHi },
  it: { ...en, ...infoEn, ...it, ...infoIt },
  ja: { ...en, ...infoEn, ...ja, ...infoJa },
  ko: { ...en, ...infoEn, ...ko, ...infoKo },
  pl: { ...en, ...infoEn, ...pl, ...infoPl },
  pt: { ...en, ...infoEn, ...pt, ...infoPt },
  ru: { ...en, ...infoEn, ...ru, ...infoRu },
  tr: { ...en, ...infoEn, ...tr, ...infoTr },
  zh: { ...en, ...infoEn, ...zh, ...infoZh },
};

const STORAGE_KEY = 'srt-tools-ui-language';

function getSystemLanguage(): string {
  if (typeof navigator !== 'undefined') {
    // Prova prima navigator.language (es: "it-IT", "en-US")
    const fullLang = navigator.language;
    const shortLang = fullLang.split('-')[0].toLowerCase();
    
    if (translations[shortLang]) {
      return shortLang;
    }
    
    // Prova navigator.languages per lingue alternative
    if (navigator.languages) {
      for (const lang of navigator.languages) {
        const short = lang.split('-')[0].toLowerCase();
        if (translations[short]) {
          return short;
        }
      }
    }
  }
  return 'en';
}

function getInitialLanguage(): string {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && translations[saved]) {
      return saved;
    }
  }
  // Usa la lingua del sistema operativo come default
  return getSystemLanguage();
}

export const currentLanguage = writable<string>(getInitialLanguage());

currentLanguage.subscribe((lang) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, lang);
  }
});

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

export const locale = derived(currentLanguage, ($lang) => {
  return (key: string, params?: Record<string, string | number>): string => {
    return translate($lang, key, params);
  };
});

export function setLanguage(lang: string): void {
  if (translations[lang]) {
    currentLanguage.set(lang);
  }
}

export function getLanguage(): string {
  return get(currentLanguage);
}

export interface DictionaryProvider {
  id: string;
  name: string;
  targetLanguage: string; // ISO 639-1 code e.g. "ja", "en", "de", "zh", "ko", "ar", "ru", "es", "fr", "it", "pt", "*"
  nativeLanguage?: string; // Optional e.g. "it", "en", "*" (if bilingual pair)
  wordUrlTemplate: string; // e.g. "https://jisho.org/search/{word}" or "https://www.wordreference.com/enit/{word}"
  sentenceUrlTemplate?: string; // e.g. "https://context.reverso.net/translation/english-italian/{sentence}"
  isPredefined: boolean;
  description?: string;
  category?: "bilingual" | "monolingual" | "specialized" | "custom";
}

export interface CustomDictionary {
  id: string;
  name: string;
  targetLanguage: string;
  nativeLanguage?: string;
  wordUrlTemplate: string;
  sentenceUrlTemplate?: string;
  description?: string;
  createdAt: number;
}

export interface FlashcardDictionaryConfig {
  enabled: boolean;
  providerId: string;
  wordUrlTemplate: string;
  sentenceUrlTemplate?: string;
  providerName: string;
}

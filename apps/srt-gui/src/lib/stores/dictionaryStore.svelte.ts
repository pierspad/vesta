import * as vestaConfig from "$lib/config/vestaConfig";
import type { CustomDictionary, DictionaryProvider } from "$lib/types/dictionaryTypes";
import { builtInDictionaries, inferDictionaryForLanguages } from "$lib/data/dictionaryProviders";

const DICTIONARY_ENABLED_KEY = "vesta-dictionary-lookup-enabled";
const CUSTOM_DICTIONARIES_KEY = "vesta-custom-dictionaries";

class DictionaryStore {
  enabled = $state(true);
  customDictionaries = $state<CustomDictionary[]>([]);

  constructor() {
    const storedEnabled = vestaConfig.getItem(DICTIONARY_ENABLED_KEY);
    // Default to true if not set
    this.enabled = storedEnabled === null || storedEnabled === undefined ? true : storedEnabled === "true";
    this.loadCustomDictionaries();
  }

  private loadCustomDictionaries() {
    try {
      const raw = vestaConfig.getItem(CUSTOM_DICTIONARIES_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          this.customDictionaries = parsed.filter(
            (d) =>
              d &&
              typeof d.id === "string" &&
              typeof d.name === "string" &&
              typeof d.wordUrlTemplate === "string"
          );
          return;
        }
      }
    } catch (e) {
      console.error("[DictionaryStore] Failed to load custom dictionaries", e);
    }
    this.customDictionaries = [];
  }

  private saveCustomDictionaries() {
    try {
      vestaConfig.setItem(CUSTOM_DICTIONARIES_KEY, JSON.stringify(this.customDictionaries));
    } catch (e) {
      console.error("[DictionaryStore] Failed to save custom dictionaries", e);
    }
  }

  toggleFeature(): boolean {
    this.enabled = !this.enabled;
    vestaConfig.setItem(DICTIONARY_ENABLED_KEY, String(this.enabled));
    return this.enabled;
  }

  setFeatureEnabled(val: boolean) {
    this.enabled = val;
    vestaConfig.setItem(DICTIONARY_ENABLED_KEY, String(this.enabled));
  }

  addCustomDictionary(data: {
    name: string;
    targetLanguage: string;
    nativeLanguage?: string;
    wordUrlTemplate: string;
    sentenceUrlTemplate?: string;
    description?: string;
  }): { success: boolean; dictionary?: CustomDictionary; error?: string } {
    const trimmedName = data.name.trim();
    const trimmedWordUrl = data.wordUrlTemplate.trim();
    const trimmedTarget = data.targetLanguage.trim().toLowerCase() || "*";

    if (!trimmedName) {
      return { success: false, error: "Nome non può essere vuoto" };
    }
    if (!trimmedWordUrl) {
      return { success: false, error: "URL del template per la parola non può essere vuoto" };
    }
    if (!trimmedWordUrl.includes("{word}")) {
      return {
        success: false,
        error: "L'URL del template per la parola deve contenere il segnaposto {word}",
      };
    }

    const newDict: CustomDictionary = {
      id: "dict_" + Date.now().toString(36) + "_" + Math.random().toString(36).substring(2, 6),
      name: trimmedName,
      targetLanguage: trimmedTarget,
      nativeLanguage: data.nativeLanguage?.trim().toLowerCase() || undefined,
      wordUrlTemplate: trimmedWordUrl,
      sentenceUrlTemplate: data.sentenceUrlTemplate?.trim() || undefined,
      description: data.description?.trim() || undefined,
      createdAt: Date.now(),
    };

    this.customDictionaries = [...this.customDictionaries, newDict];
    this.saveCustomDictionaries();
    return { success: true, dictionary: newDict };
  }

  removeCustomDictionary(id: string): boolean {
    const prevLen = this.customDictionaries.length;
    this.customDictionaries = this.customDictionaries.filter((d) => d.id !== id);
    if (this.customDictionaries.length !== prevLen) {
      this.saveCustomDictionaries();
      return true;
    }
    return false;
  }

  getDictionaryById(id: string): DictionaryProvider | undefined {
    if (!id) return undefined;
    if (id.startsWith("custom_")) {
      const realId = id.replace(/^custom_/, "");
      const custom = this.customDictionaries.find((c) => c.id === realId || `dict_${c.id}` === realId || c.id === id);
      if (custom) {
        return {
          id: `custom_${custom.id}`,
          name: `⭐ ${custom.name}`,
          targetLanguage: custom.targetLanguage,
          nativeLanguage: custom.nativeLanguage,
          wordUrlTemplate: custom.wordUrlTemplate,
          sentenceUrlTemplate: custom.sentenceUrlTemplate,
          isPredefined: false,
          category: "custom",
          description: custom.description,
        };
      }
    }
    return builtInDictionaries.find((d) => d.id === id);
  }

  resolveProvider(targetLang: string, nativeLang = "it", overrideProviderId?: string): DictionaryProvider {
    if (overrideProviderId) {
      const found = this.getDictionaryById(overrideProviderId);
      if (found) return found;
    }
    return inferDictionaryForLanguages(targetLang, nativeLang);
  }
}

export const dictionaryStore = new DictionaryStore();

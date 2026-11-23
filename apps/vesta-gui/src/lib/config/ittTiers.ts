import type { ApiProviderId } from "$lib/config/apiKeys";
import * as vestaConfig from "$lib/config/vestaConfig";
import { normalizeTierList } from "$lib/config/tierNormalization";
import { newTierId, newTierEntryId, type Tier } from "$lib/config/translationTiers";

const ITT_TIERS_KEY = "vesta-itt-tiers";
const DEFAULT_OCR_MODEL_ID = "paddleocr-v4-mobile-multi";
export const ITT_TIERS_UPDATED_EVENT = "vesta:itt-tiers-updated";

/** Loads the configured ITT (Image-to-Text / Vision OCR) priority tiers. */
export function loadIttTiers(): Tier[] {
  const defaultOcrModel = vestaConfig.getItem("vesta-default-ocr-model") || DEFAULT_OCR_MODEL_ID;
  try {
    const raw = vestaConfig.getItem(ITT_TIERS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      const normalized = normalizeTierList(parsed, newTierId, newTierEntryId, (provider) => provider as ApiProviderId);
      if (normalized && normalized.length > 0) {
        return normalized.map((tier) => ({
          ...tier,
          entries: tier.entries.map((entry) =>
            entry.provider === "local_ocr" || entry.apiKeyId === "local_ocr"
              ? { ...entry, provider: "local_ocr", apiKeyId: "local_ocr", model: entry.model || defaultOcrModel }
              : entry,
          ),
        }));
      }
    }
  } catch {
    /* ignore */
  }
  return [
    {
      id: newTierId(),
      entries: [
        {
          id: newTierEntryId(),
          provider: "local_ocr",
          apiKeyId: "local_ocr",
          model: defaultOcrModel,
        },
      ],
    },
  ];
}

/** Saves the configured ITT priority tiers and notifies listeners. */
export function saveIttTiers(tiers: Tier[]): void {
  vestaConfig.setItem(ITT_TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(ITT_TIERS_UPDATED_EVENT));
  }
}

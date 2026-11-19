import type { ApiProviderId } from "$lib/config/apiKeys";
import * as vestaConfig from "$lib/config/vestaConfig";
import { normalizeTierList } from "$lib/config/tierNormalization";
import { newTierId, newTierEntryId, type Tier } from "$lib/config/translationTiers";

const ITT_TIERS_KEY = "vesta-itt-tiers";
export const ITT_TIERS_UPDATED_EVENT = "vesta:itt-tiers-updated";

/** Loads the configured ITT (Image-to-Text / Vision OCR) priority tiers. */
export function loadIttTiers(): Tier[] {
  try {
    const raw = vestaConfig.getItem(ITT_TIERS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      const normalized = normalizeTierList(parsed, newTierId, newTierEntryId, (provider) => provider as ApiProviderId);
      if (normalized) return normalized;
    }
  } catch {
    /* ignore */
  }
  return [];
}

/** Saves the configured ITT priority tiers and notifies listeners. */
export function saveIttTiers(tiers: Tier[]): void {
  vestaConfig.setItem(ITT_TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(ITT_TIERS_UPDATED_EVENT));
  }
}

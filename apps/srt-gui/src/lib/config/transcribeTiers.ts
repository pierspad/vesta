import { newTierId, newTierEntryId } from "$lib/config/translationTiers";
import * as vestaConfig from "$lib/config/vestaConfig";

// ─── Transcribe Tiers ──────────────────────────────────────────────────────────
// Stesso schema di failover di translationTiers.ts, ma per la trascrizione
// cloud; riusa newTierId/newTierEntryId (gli id non hanno bisogno di essere
// distinti per dominio).

export interface TranscribeTierEntry {
  id: string;
  provider: string; // "local" | "groq" | "openai" | "deepgram" | "assemblyai" | "custom"
  apiKeyId: string;
  model: string;
  rpm?: number;
  maxRequests?: number;
}

export interface TranscribeTier {
  id: string;
  entries: TranscribeTierEntry[];
}

const TRANSCRIBE_TIERS_KEY = "vesta-transcribe-tiers";
export const TRANSCRIBE_TIERS_UPDATED_EVENT = "vesta:transcribe-tiers-updated";

export function loadTranscribeTiers(): TranscribeTier[] {
  try {
    const raw = vestaConfig.getItem(TRANSCRIBE_TIERS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed
          .filter((t) => t && Array.isArray(t.entries))
          .map((t) => ({
            id: typeof t.id === "string" ? t.id : newTierId(),
            entries: (t.entries as any[])
              .filter((e) => e && typeof e.provider === "string")
              .map((e) => ({
                id: typeof e.id === "string" ? e.id : newTierEntryId(),
                provider: e.provider,
                apiKeyId: typeof e.apiKeyId === "string" ? e.apiKeyId : "",
                model: typeof e.model === "string" ? e.model : "",
                rpm: typeof e.rpm === "number" && e.rpm > 0 ? e.rpm : undefined,
                maxRequests: typeof e.maxRequests === "number" && e.maxRequests > 0 ? e.maxRequests : undefined,
              })),
          }));
      }
    }
  } catch {
    /* ignore */
  }
  // Default: one tier with local base model
  return [
    {
      id: newTierId(),
      entries: [
        {
          id: newTierEntryId(),
          provider: "local",
          apiKeyId: "",
          model: "base",
        }
      ]
    }
  ];
}

export function saveTranscribeTiers(tiers: TranscribeTier[]): void {
  vestaConfig.setItem(TRANSCRIBE_TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(TRANSCRIBE_TIERS_UPDATED_EVENT));
  }
}

export function transcribeTiersHaveUsableEntries(tiers: TranscribeTier[]): boolean {
  return tiers.some((t) => t.entries.some((e) => e.model && e.model.trim().length > 0));
}

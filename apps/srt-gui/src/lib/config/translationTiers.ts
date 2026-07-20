import type { ApiProviderId } from "$lib/config/apiKeys";
import * as vestaConfig from "$lib/config/vestaConfig";

// ─── Translation Tiers (priority list with automatic failover) ────────────────
//
// Una "tier list" è una lista ordinata di tier. tiers[0] è il tier a priorità
// massima. Ogni tier contiene una o più entry; all'interno del tier le entry
// vengono usate in round-robin, e quando un intero tier esaurisce i limiti si
// passa automaticamente al tier successivo (failover, gestito dal backend Rust).
//
// Una entry punta a una API key salvata (apiKeyId) e a un modello specifico:
// così la stessa key può comparire più volte con modelli diversi nello stesso
// tier, sfruttando quote separate per modello con un'unica chiave.

export interface TierEntry {
  id: string;
  provider: ApiProviderId;
  /** Riferimento a ApiKeyConfig.id. Vuoto per provider locali senza key. */
  apiKeyId: string;
  /** Id del modello da chiamare. */
  model: string;
  /** Override opzionale RPM (richieste/minuto). */
  rpm?: number;
  /** Budget opzionale di richieste per run. */
  maxRequests?: number;
}

export interface Tier {
  id: string;
  entries: TierEntry[];
}

const TIERS_KEY = "vesta-translate-tiers";
export const TIERS_UPDATED_EVENT = "vesta:tiers-updated";

export function newTierId(): string {
  return `tier-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export function newTierEntryId(): string {
  return `te-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export function loadTiers(): Tier[] {
  try {
    const raw = vestaConfig.getItem(TIERS_KEY);
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
                provider: e.provider as ApiProviderId,
                apiKeyId: typeof e.apiKeyId === "string" ? e.apiKeyId : "",
                model: typeof e.model === "string" ? e.model : "",
                rpm: typeof e.rpm === "number" && e.rpm > 0 ? e.rpm : undefined,
                maxRequests:
                  typeof e.maxRequests === "number" && e.maxRequests > 0
                    ? e.maxRequests
                    : undefined,
              })),
          }));
      }
    }
  } catch {
    /* ignore */
  }
  return [];
}

export function saveTiers(tiers: Tier[]): void {
  vestaConfig.setItem(TIERS_KEY, JSON.stringify(tiers));
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(TIERS_UPDATED_EVENT));
  }
}

/** True se almeno una entry ha un modello valido (tier configurabili e usabili). */
export function tiersHaveUsableEntries(tiers: Tier[]): boolean {
  return tiers.some((t) => t.entries.some((e) => e.model && e.model.trim().length > 0));
}

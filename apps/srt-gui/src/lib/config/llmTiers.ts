/**
 * Helper condivisi per il sistema LLM a tier (Translate, Annotate/Refine, …).
 *
 * Un solo punto di verità per:
 *  - costruire il payload `tiers` da passare ai comandi Tauri
 *    (risolvendo le API key referenziate per id);
 *  - verificare che almeno un endpoint sia utilizzabile prima di avviare
 *    un run (ping dei server locali inclusi).
 *
 * I motivi di indisponibilità sono codici: ogni tab li mappa sulle proprie
 * stringhe i18n.
 */

import { supportsCapability, type ApiKeyConfig } from "$lib/config/apiKeys";
import { type Tier } from "$lib/config/translationTiers";
import { providers } from "$lib/config/llmProviders";
import { fetchModelsFromEndpoint } from "$lib/services/modelDiscovery";
import { t } from "$lib/i18n";

/** Entry serializzata per i comandi Tauri (contratto serde con `srt_translate::TierEntry`). */
export interface TierEntryPayload {
  provider: string;
  model: string;
  api_key: string | null;
  api_url: string | null;
  rpm: number | null;
  max_requests: number | null;
}

/**
 * Costruisce il payload a tier risolvendo le API key referenziate.
 * Ritorna `null` se nessuna entry è utilizzabile.
 */
export function buildTiersPayload(
  tiers: Tier[],
  apiKeys: ApiKeyConfig[],
): TierEntryPayload[][] | null {
  const out: TierEntryPayload[][] = [];
  for (const tier of tiers) {
    const entries: TierEntryPayload[] = [];
    for (const e of tier.entries) {
      if (!e.model || !e.model.trim()) continue;
      const key = apiKeys.find((k) => k.id === e.apiKeyId) || (e.provider === "local" ? apiKeys.find(k => k.id === "local") : undefined);
      if (key?.capabilities && !supportsCapability(key, "llm")) continue;
      const provider = e.provider || key?.apiType || "google";
      const apiKeyVal = key?.apiKey?.trim() || null;
      const apiUrl = key?.apiUrl?.trim() || providers[provider]?.defaultApiUrl || null;
      // Le entry remote senza key valida vengono saltate.
      if (!apiKeyVal && provider !== "local" && provider !== "custom") continue;
      entries.push({
        provider,
        model: e.model.trim(),
        api_key: apiKeyVal,
        api_url: apiUrl,
        rpm: e.rpm ?? null,
        max_requests: e.maxRequests ?? null,
      });
    }
    if (entries.length > 0) out.push(entries);
  }
  return out.length > 0 ? out : null;
}

/** Motivo per cui i tier non sono utilizzabili in questo momento. */
export type TiersUnavailableReason =
  | "noneConfigured"
  | "localOffline"
  | "keyMissing"
  | "incomplete"
  | "unknown";

export type TiersAvailability =
  | { available: true }
  | { available: false; reason: TiersUnavailableReason };

/**
 * Verifica che almeno un endpoint dei tier sia utilizzabile:
 * i provider locali/custom vengono pingati (`/models`), quelli remoti
 * richiedono una key valida.
 */
export async function checkTiersAvailability(
  tiers: Tier[],
  apiKeys: ApiKeyConfig[],
): Promise<TiersAvailability> {
  const configuredEntries = tiers.flatMap((tier) =>
    tier.entries.filter((e) => e.model && e.model.trim()),
  );
  if (configuredEntries.length === 0) {
    return { available: false, reason: "noneConfigured" };
  }

  let hasOnlineLocal = false;
  let hasValidRemote = false;
  let hasKeyMissing = false;
  let hasOfflineLocal = false;
  let hasIncomplete = false;
  // Multiple configured models may use the same local/custom server. Probe
  // each endpoint once per availability check while preserving per-entry
  // credential and capability validation above.
  const probes = new Map<string, Promise<boolean>>();

  await Promise.all(
    configuredEntries.map(async (e) => {
      const key = apiKeys.find((k) => k.id === e.apiKeyId) || (e.provider === "local" ? apiKeys.find(k => k.id === "local") : undefined);
      if (key?.capabilities && !supportsCapability(key, "llm")) { hasIncomplete = true; return; }
      const provider = e.provider || key?.apiType || "google";
      const apiKeyVal = key?.apiKey?.trim() || null;

      if (provider === "local" || provider === "custom") {
        const apiUrl = key?.apiUrl?.trim() || providers[provider]?.defaultApiUrl || "";
        if (!apiUrl.trim()) {
          hasIncomplete = true;
          return;
        }
        const probeKey = JSON.stringify([apiUrl, apiKeyVal || ""]);
        let probe = probes.get(probeKey);
        if (!probe) {
          probe = fetchModelsFromEndpoint(apiUrl, apiKeyVal || "", 2000)
            .then((models) => Boolean(models && models.length > 0))
            .catch((error) => {
              // A reachable server may omit discovery while supporting manual model IDs.
              return Boolean(error && typeof error === "object" && "status" in error && [404, 405].includes(Number(error.status)));
            });
          probes.set(probeKey, probe);
        }
        if (await probe) hasOnlineLocal = true;
        else hasOfflineLocal = true;
      } else if (!apiKeyVal) {
        hasKeyMissing = true;
      } else {
        hasValidRemote = true;
      }
    }),
  );

  if (hasOnlineLocal || hasValidRemote) return { available: true };
  if (hasOfflineLocal && !hasKeyMissing && !hasIncomplete) {
    return { available: false, reason: "localOffline" };
  }
  if (hasKeyMissing && !hasOfflineLocal && !hasIncomplete) {
    return { available: false, reason: "keyMissing" };
  }
  if (hasIncomplete && !hasOfflineLocal && !hasKeyMissing) {
    return { available: false, reason: "incomplete" };
  }
  if (hasOfflineLocal) return { available: false, reason: "localOffline" };
  return { available: false, reason: "unknown" };
}

/**
 * Traduce un `TiersUnavailableReason` nel messaggio da mostrare in UI.
 * RefineTab e TranslateTab finivano per ripetere lo stesso switch statement
 * -- in pratica risolvono sulle stesse chiavi i18n, quindi la mappatura è
 * condivisa qui; solo il fallback del caso "unknown" resta per-chiamante
 * (ogni tab ha un messaggio d'errore generico diverso).
 */
export function tiersUnavailableMessage(
  reason: TiersUnavailableReason,
  fallbackKey: string = "tiers.noneConfigured",
): string {
  switch (reason) {
    case "noneConfigured":
      return t("tiers.noneConfigured");
    case "localOffline":
      return t("settings.llmConfigIncompleteDescLocalOffline");
    case "keyMissing":
      return t("settings.llmConfigIncompleteDescKey");
    case "incomplete":
      return t("settings.llmConfigIncompleteDescCustomEmpty");
    default:
      return t(fallbackKey);
  }
}

/** Conteggi per il riepilogo nella UI ("N tier · M endpoint"). */
export function countTiersAndEndpoints(tiers: Tier[]): { tiers: number; endpoints: number } {
  const usable = tiers.filter((tier) => tier.entries.some((e) => e.model && e.model.trim()));
  return {
    tiers: usable.length,
    endpoints: usable.reduce(
      (sum, tier) => sum + tier.entries.filter((e) => e.model && e.model.trim()).length,
      0,
    ),
  };
}

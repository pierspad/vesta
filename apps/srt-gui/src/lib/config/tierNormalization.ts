export interface NormalizedTierEntry<Provider extends string> {
  id: string;
  provider: Provider;
  apiKeyId: string;
  model: string;
  rpm?: number;
  maxRequests?: number;
}

export interface NormalizedTier<Provider extends string> {
  id: string;
  entries: NormalizedTierEntry<Provider>[];
}

/** Normalize the shared persisted tier shape; domain defaults remain caller-owned. */
export function normalizeTierList<Provider extends string>(
  value: unknown,
  newTierId: () => string,
  newEntryId: () => string,
  parseProvider: (provider: string) => Provider,
): NormalizedTier<Provider>[] | null {
  if (!Array.isArray(value)) return null;

  return value
    .filter((tier): tier is Record<string, unknown> => Boolean(tier) && typeof tier === "object" && Array.isArray((tier as Record<string, unknown>).entries))
    .map((tier) => ({
      id: typeof tier.id === "string" ? tier.id : newTierId(),
      entries: (tier.entries as unknown[])
        .filter((entry): entry is Record<string, unknown> => Boolean(entry) && typeof entry === "object" && typeof (entry as Record<string, unknown>).provider === "string")
        .map((entry) => ({
          id: typeof entry.id === "string" ? entry.id : newEntryId(),
          provider: parseProvider(entry.provider as string),
          apiKeyId: typeof entry.apiKeyId === "string" ? entry.apiKeyId : "",
          model: typeof entry.model === "string" ? entry.model : "",
          rpm: typeof entry.rpm === "number" && entry.rpm > 0 ? entry.rpm : undefined,
          maxRequests: typeof entry.maxRequests === "number" && entry.maxRequests > 0 ? entry.maxRequests : undefined,
        })),
    }));
}

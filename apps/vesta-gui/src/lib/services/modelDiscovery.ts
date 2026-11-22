import { fetchModelCatalog, type CatalogPagination } from "$lib/services/modelCatalog";

export interface DiscoveredModel {
  id: string;
  name: string;
}

export function extractModelsFromPayload(payload: unknown): DiscoveredModel[] {
  const groups: unknown[][] = [];

  if (Array.isArray(payload)) {
    groups.push(payload);
  } else if (payload && typeof payload === "object") {
    const record = payload as Record<string, unknown>;

    if (Array.isArray(record.data)) {
      groups.push(record.data);
    }
    if (Array.isArray(record.models)) {
      groups.push(record.models);
    }

    const nestedData = record.data;
    if (nestedData && typeof nestedData === "object") {
      const nestedRecord = nestedData as Record<string, unknown>;
      if (Array.isArray(nestedRecord.models)) {
        groups.push(nestedRecord.models);
      }
    }
  }

  const seen = new Set<string>();
  const models: DiscoveredModel[] = [];

  for (const candidates of groups) {
    for (const candidate of candidates) {
      let id = "";
      let name = "";

      if (typeof candidate === "string") {
        id = candidate.trim();
        name = id;
      } else if (candidate && typeof candidate === "object") {
        const record = candidate as Record<string, unknown>;
        const rawId = [record.id, record.name, record.model]
          .find((value) => typeof value === "string" && value.trim().length > 0);
        const rawName = [record.displayName, record.display_name, record.label, record.name, record.id]
          .find((value) => typeof value === "string" && value.trim().length > 0);

        if (typeof rawId === "string") {
          id = rawId.trim();
        }
        if (typeof rawName === "string") {
          name = rawName.trim();
        }
      }

      if (!id || seen.has(id)) continue;
      seen.add(id);
      models.push({ id, name: name || id });
    }
  }

  return models;
}

/** Normalize Google's catalog for both settings discovery and managed-key probing. */
export function extractGoogleModels(payload: unknown, requireCapability = false): DiscoveredModel[] {
  const entries = payload && typeof payload === "object" ? (payload as Record<string, unknown>).models : undefined;
  if (!Array.isArray(entries)) return [];
  const seen = new Set<string>();
  const models: DiscoveredModel[] = [];
  for (const entry of entries) {
    if (!entry || typeof entry !== "object") continue;
    const record = entry as Record<string, unknown>;
    const methods = record.supportedGenerationMethods;
    if (Array.isArray(methods) ? !methods.includes("generateContent") : requireCapability) continue;
    const rawId = [record.name, record.id].find(value => typeof value === "string" && value.trim());
    if (typeof rawId !== "string") continue;
    const id = rawId.trim().replace(/^models\//, "").trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    const name = typeof record.displayName === "string" ? record.displayName.trim() : "";
    models.push({ id, name: name || id });
  }
  return models;
}

export function buildModelsUrl(baseUrl: string) {
  let url = baseUrl.trim().replace(/\/+$/, "");

  if (!url) return url;

  // LM Studio exposes /v1/models. Only repair loopback URLs: remote
  // providers such as OpenRouter require the /api prefix.
  try {
    if (["localhost", "127.0.0.1", "[::1]"].includes(new URL(url).hostname)) {
      url = url.replace(/\/api(?=\/v1(?:\/models)?$)/, "");
    }
  } catch {
    throw new Error("Invalid endpoint URL");
  }

  if (url.endsWith("/models")) {
    return url;
  }

  return url.endsWith("/v1") ? `${url}/models` : `${url}/v1/models`;
}

async function fetchModelsWithHeaders(
  url: string,
  headers: Record<string, string>,
  timeoutMs: number,
  pagination: CatalogPagination = "none",
): Promise<DiscoveredModel[]> {
  const data = await fetchModelCatalog(url, {
    method: "GET",
    headers,
    timeoutMs,
    // `url` è sempre derivato dall'endpoint configurato dall'utente (provider
    // local/custom o apiUrl personalizzato di una key), non un host cablato:
    // la whitelist statica non si applica.
    allowCustomHost: true,
  }, pagination);
  const models = extractModelsFromPayload(data);
  if (models.length === 0) throw new Error("No models found");
  return models;
}

export async function fetchModelsFromEndpoint(
  baseUrl: string,
  apiKey = "",
  timeoutMs = 8000,
): Promise<DiscoveredModel[]> {
  const url = buildModelsUrl(baseUrl);
  if (!url) throw new Error("Missing endpoint");

  const headers: Record<string, string> = {
    Accept: "application/json",
  };
  if (apiKey.trim()) {
    headers.Authorization = `Bearer ${apiKey.trim()}`;
  }

  return fetchModelsWithHeaders(url, headers, timeoutMs);
}

/**
 * Scopre i modelli disponibili per un provider, gestendo le differenze di API:
 *  - Google Gemini: GET {base}/models con x-goog-api-key, filtra a generateContent, rimuove "models/".
 *  - Anthropic Claude: GET {base}/models con x-api-key e anthropic-version.
 *  - Tutto il resto (OpenAI-compatible): GET {base}/models con Bearer.
 *
 * Pensata per essere chiamata a runtime così che nuovi modelli compaiano senza
 * dover aggiornare l'app.
 */
export async function discoverModels(
  provider: string,
  apiKey: string,
  apiUrl: string,
  timeoutMs = 8000,
): Promise<DiscoveredModel[]> {
  const p = provider.toLowerCase();

  if (p === "google" || p === "gemini") {
    const base = (apiUrl || "https://generativelanguage.googleapis.com/v1beta")
      .trim()
      .replace(/\/+$/, "");
    const url = `${base}/models`;
    const data = await fetchModelCatalog(url, {
      method: "GET",
      headers: { Accept: "application/json", "x-goog-api-key": apiKey.trim() },
      timeoutMs,
      // Come sopra: `base` può essere l'apiUrl personalizzato dell'utente
      // per un proxy Gemini-compatible.
      allowCustomHost: true,
    }, "google");
    const models = extractGoogleModels(data);
    if (models.length === 0) throw new Error("No models found");
    return models;
  }

  if (p === "anthropic") {
    const base = (apiUrl || "https://api.anthropic.com/v1").trim().replace(/\/+$/, "");
    const url = buildModelsUrl(base || "https://api.anthropic.com/v1");
    return fetchModelsWithHeaders(
      url,
      {
        Accept: "application/json",
        "x-api-key": apiKey.trim(),
        "anthropic-version": "2023-06-01",
      },
      timeoutMs,
      "anthropic",
    );
  }

  // OpenAI-compatible (groq, openrouter, mistral, github, nvidia, local, custom).
  return fetchModelsFromEndpoint(apiUrl, apiKey, timeoutMs);
}

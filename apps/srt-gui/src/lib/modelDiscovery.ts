import { fetch as tauriFetch } from "@tauri-apps/plugin-http";

export interface DiscoveredModel {
  id: string;
  name: string;
}

export function extractModelsFromPayload(payload: unknown): DiscoveredModel[] {
  const candidates: unknown[] = [];

  if (Array.isArray(payload)) {
    candidates.push(...payload);
  } else if (payload && typeof payload === "object") {
    const record = payload as Record<string, unknown>;

    if (Array.isArray(record.data)) {
      candidates.push(...record.data);
    }
    if (Array.isArray(record.models)) {
      candidates.push(...record.models);
    }

    const nestedData = record.data;
    if (nestedData && typeof nestedData === "object") {
      const nestedRecord = nestedData as Record<string, unknown>;
      if (Array.isArray(nestedRecord.models)) {
        candidates.push(...nestedRecord.models);
      }
    }
  }

  const seen = new Set<string>();
  const models: DiscoveredModel[] = [];

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
      const rawName = [record.displayName, record.label, record.name, record.id]
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

  return models;
}

export function buildModelsUrl(baseUrl: string) {
  let url = baseUrl.trim().replace(/\/+$/, "");

  if (!url) return url;

  // LM Studio exposes /v1/models; users sometimes paste /api/v1.
  url = url.replace(/\/api(?=\/v1(?:\/models)?$)/, "");

  if (url.endsWith("/models")) {
    return url;
  }

  return url.endsWith("/v1") ? `${url}/models` : `${url}/v1/models`;
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

  const resp = await tauriFetch(url, {
    method: "GET",
    headers,
    signal: AbortSignal.timeout(timeoutMs),
  });

  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

  const responseText = await resp.text();
  let data: unknown = null;
  if (responseText.trim().length > 0) {
    try {
      data = JSON.parse(responseText);
    } catch {
      throw new Error("Invalid JSON response");
    }
  }

  const models = extractModelsFromPayload(data);
  if (models.length === 0) throw new Error("No models found");
  return models;
}

/**
 * Scopre i modelli disponibili per un provider, gestendo le differenze di API:
 *  - Google Gemini: GET {base}/models?key=KEY, filtra a generateContent, rimuove "models/".
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
    const url = `${base}/models?key=${encodeURIComponent(apiKey.trim())}`;
    const resp = await tauriFetch(url, {
      method: "GET",
      headers: { Accept: "application/json" },
      signal: AbortSignal.timeout(timeoutMs),
    });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    const data = (await resp.json()) as { models?: any[] };
    const arr = Array.isArray(data?.models) ? data.models : [];
    const seen = new Set<string>();
    const models: DiscoveredModel[] = [];
    for (const m of arr) {
      const methods = m?.supportedGenerationMethods;
      if (Array.isArray(methods) && !methods.includes("generateContent")) continue;
      const id = String(m?.name || m?.id || "").replace(/^models\//, "").trim();
      if (!id || seen.has(id)) continue;
      seen.add(id);
      models.push({ id, name: String(m?.displayName || id).trim() || id });
    }
    if (models.length === 0) throw new Error("No models found");
    return models;
  }

  // OpenAI-compatible (groq, openrouter, mistral, github, nvidia, local, custom).
  return fetchModelsFromEndpoint(apiUrl, apiKey, timeoutMs);
}

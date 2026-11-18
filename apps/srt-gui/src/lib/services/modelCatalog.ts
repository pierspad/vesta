import { fetch as tauriFetch, type TauriFetchOptions } from "$lib/services/tauriHttp";

export type CatalogPagination = "none" | "google" | "anthropic";

async function parseResponse(response: Awaited<ReturnType<typeof tauriFetch>>): Promise<unknown> {
  try {
    if (typeof response.text === "function") {
      const text = await response.text();
      return text.trim() ? JSON.parse(text) : null;
    }
    return typeof response.json === "function" ? await response.json() : null;
  } catch {
    throw new Error("Invalid JSON response");
  }
}

/** Read complete provider catalogs without trusting remote pagination URLs or unbounded cursors. */
export async function fetchModelCatalog(
  url: string,
  options: TauriFetchOptions,
  pagination: CatalogPagination = "none",
  fetch: typeof tauriFetch = tauriFetch,
): Promise<unknown> {
  const deadline = Date.now() + (options.timeoutMs ?? 15000);
  const seen = new Set<string>();
  const items: unknown[] = [];
  let nextUrl = url;
  for (let page = 0; page < 50; page++) {
    const remaining = deadline - Date.now();
    if (remaining <= 0) throw new Error("Model discovery timed out");
    const response = await fetch(nextUrl, {
      ...options,
      method: "GET",
      ...(page > 0 ? { timeoutMs: remaining } : {}),
    });
    if (!response.ok) throw Object.assign(new Error(`HTTP ${response.status}`), { status: response.status });
    const payload = await parseResponse(response);
    if (pagination === "none" || !payload || typeof payload !== "object" || Array.isArray(payload)) return payload;
    const record = payload as Record<string, unknown>;
    const field = pagination === "google" ? "models" : "data";
    const entries = record[field];
    if (!Array.isArray(entries)) throw new Error("Invalid model catalog");
    if (entries.length > 10000 - items.length) throw new Error("Model catalog exceeds the entry limit");
    for (const entry of entries) items.push(entry);
    const cursor = pagination === "google" ? record.nextPageToken : record.has_more === true ? record.last_id : undefined;
    if (cursor === undefined || cursor === null || cursor === "") {
      if (pagination === "anthropic" && record.has_more === true) throw new Error("Missing model pagination cursor");
      return { ...record, [field]: items };
    }
    if (typeof cursor !== "string" || seen.has(cursor)) throw new Error("Invalid or repeated model pagination cursor");
    seen.add(cursor);
    const next = new URL(url);
    next.searchParams.set(pagination === "google" ? "pageToken" : "after_id", cursor);
    nextUrl = next.toString();
  }
  throw new Error("Model catalog exceeds the page limit");
}

import { fetch as tauriFetch } from "$lib/services/tauriHttp";
import { extractModelsFromPayload, type DiscoveredModel } from "$lib/services/modelDiscovery";

/** Provider-specific discovery and accessibility probing, independent of tab state/cache. */
export async function discoverManagedProviderModels(
  family: "google" | "groq",
  apiKey: string,
  fetch: typeof tauriFetch = tauriFetch,
): Promise<DiscoveredModel[]> {
  let url = "";
  let headers: Record<string, string> = { "Accept": "application/json" };

  if (family === "google") {
    url = "https://generativelanguage.googleapis.com/v1beta/models";
    headers["x-goog-api-key"] = apiKey;
  } else if (family === "groq") {
    url = "https://api.groq.com/openai/v1/models";
    headers["Authorization"] = `Bearer ${apiKey}`;
  }

  const resp = await fetch(url, { method: "GET", headers });
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

  const responseText = await resp.text();
  let data: unknown = null;
  if (responseText.trim().length > 0) {
     data = JSON.parse(responseText);
  }

  let models = extractModelsFromPayload(data);
  if (models.length === 0) throw new Error("No models found dynamically");

  // For Google, strip 'models/' prefix from the ID, use displayName if available,
  // and filter to only show models that support text generation (generateContent)
  if (family === "google") {
    const rawModels = data && typeof data === "object"
      ? (data as Record<string, unknown>).models : undefined;
    if (Array.isArray(rawModels)) {
      models = extractModelsFromPayload(rawModels.filter((model: unknown) => {
        if (!model || typeof model !== "object") return false;
        const methods = (model as Record<string, unknown>).supportedGenerationMethods;
        return Array.isArray(methods) && methods.includes("generateContent");
      }));
    }
    const seen = new Set<string>();
    models = models.flatMap(({ id, name }) => {
      const normalized = id.replace(/^models\//, "").trim();
      if (!normalized || seen.has(normalized)) return [];
      seen.add(normalized);
      return [{ id: normalized, name: name === id ? normalized : name }];
    });
  }

  if (models.length === 0) throw new Error("No usable text generation models found");

  // For Google, probe each model with generateContent to check actual accessibility.
  if (family === "google") {
    // Bound the request burst and preserve catalog order despite out-of-order replies.
    const accessible = new Set<number>();
    let nextIndex = 0;
    const probeNext = async () => {
      while (nextIndex < models.length) {
        const index = nextIndex++;
        const model = models[index];
        try {
          const probeResp = await fetch(
            `https://generativelanguage.googleapis.com/v1beta/models/${encodeURIComponent(model.id)}:generateContent`,
            {
              method: "POST",
              headers: {
                "Content-Type": "application/json",
                "x-goog-api-key": apiKey,
              },
              body: JSON.stringify({
                contents: [{ parts: [{ text: "hi" }] }],
                generationConfig: { maxOutputTokens: 1 },
              }),
              timeoutMs: 8000,
            },
          );
          if (probeResp.ok) accessible.add(index);
        } catch {
          // A failed probe must not stop discovery of the remaining models.
        }
      }
    };
    await Promise.all(Array.from({ length: Math.min(4, models.length) }, probeNext));
    const accessibleModels = models.filter((_, index) => accessible.has(index));
    if (accessibleModels.length > 0) {
      models = accessibleModels;
    }
    // If all probes fail, keep original list as fallback
  }

  return models;
}

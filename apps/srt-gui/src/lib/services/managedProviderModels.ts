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
    url = `https://generativelanguage.googleapis.com/v1beta/models?key=${apiKey}`;
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
    if (data && typeof data === "object" && Array.isArray((data as any).models)) {
      models = (data as any).models
        .filter((m: any) => {
          // Only show models that support generateContent (text generation)
          const methods: string[] = m.supportedGenerationMethods || [];
          return methods.includes("generateContent");
        })
        .map((m: any) => {
           const id = m.name?.replace("models/", "") || m.id;
           const name = m.displayName || id;
           return { id, name };
        });
    } else {
         models = models.map(m => {
             const id = m.id.replace("models/", "");
             return { id, name: m.name.replace("models/", "") };
         });
    }
  }

  if (models.length === 0) throw new Error("No usable text generation models found");

  // For Google, probe each model with generateContent to check actual accessibility.
  if (family === "google") {
    const probeResults = await Promise.allSettled(
      models.map(async (m) => {
        const probeUrl = `https://generativelanguage.googleapis.com/v1beta/models/${m.id}:generateContent?key=${apiKey}`;
        const probeResp = await fetch(probeUrl, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            contents: [{ parts: [{ text: "hi" }] }],
            generationConfig: { maxOutputTokens: 1 },
          }),
          timeoutMs: 8000,
        });
        if (!probeResp.ok) {
          throw new Error(`HTTP ${probeResp.status}`);
        }
        return m;
      })
    );
    const accessibleModels = probeResults
      .filter((r): r is PromiseFulfilledResult<{ id: string; name: string }> => r.status === "fulfilled")
      .map((r) => r.value);
    if (accessibleModels.length > 0) {
      models = accessibleModels;
    }
    // If all probes fail, keep original list as fallback
  }

  return models;
}

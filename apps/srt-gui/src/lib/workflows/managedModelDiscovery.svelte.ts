import type { DiscoveredModel } from "$lib/services/modelDiscovery";
import { discoverManagedProviderModels } from "$lib/services/managedProviderModels";
import { credentialFingerprint } from "./credentialIdentity";

interface ModelCache { getItem(key: string): string | null; setItem(key: string, value: string): void }
const CACHE_VERSION = 2;
const CACHE_TTL_MS = 24 * 60 * 60 * 1000;
export class ManagedModelDiscovery {
  models = $state<DiscoveredModel[]>([]);
  loading = $state(false);
  private version = 0;
  private identity = "";
  private disposed = false;
  constructor(
    private readonly cache: ModelCache,
    private readonly discover = discoverManagedProviderModels,
    private readonly onError: (error: unknown) => void = console.warn,
  ) {}
  reset() {
    this.version += 1;
    this.identity = "";
    this.models = [];
    this.loading = false;
  }
  async load(family: string, apiKey: string, force = false) {
    if (this.disposed) return;
    const identity = `${family}:${apiKey}`;
    if (!force && this.identity === identity) return;
    this.reset();
    this.identity = identity;
    if ((family !== "google" && family !== "groq") || !apiKey) return;
    const version = this.version;
    // Hash credentials for storage identity; never truncate them into collision-prone keys.
    let digest: string | null = null;
    try { digest = await credentialFingerprint(apiKey); } catch { /* cache unavailable; continue live discovery */ }
    if (this.disposed || version !== this.version) return;
    const key = digest ? `vesta-dynamic-models-v${CACHE_VERSION}-${family}-${digest}` : null;
    if (!force && key) {
      try {
        const cached = JSON.parse(this.cache.getItem(key) || "null");
        if (cached?.version === CACHE_VERSION && Number.isFinite(cached.ts) &&
          Date.now() - cached.ts >= 0 && Date.now() - cached.ts < CACHE_TTL_MS &&
          Array.isArray(cached.models) && cached.models.length > 0 && cached.models.every((model: any) =>
            model && typeof model.id === "string" && typeof model.name === "string")) {
          this.models = cached.models;
          return;
        }
      } catch { /* A malformed/unavailable cache must not prevent live discovery. */ }
    }
    this.loading = true;
    try {
      const models = await this.discover(family, apiKey);
      if (this.disposed || version !== this.version) return;
      this.models = models;
      if (key) this.cache.setItem(key, JSON.stringify({ version: CACHE_VERSION, ts: Date.now(), models }));
    } catch (error) {
      if (!this.disposed && version === this.version) this.onError(error);
    } finally {
      if (!this.disposed && version === this.version) this.loading = false;
    }
  }
  dispose() { this.disposed = true; this.version += 1; }
}

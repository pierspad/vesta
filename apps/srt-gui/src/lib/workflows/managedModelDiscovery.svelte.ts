import type { DiscoveredModel } from "$lib/services/modelDiscovery";
import { discoverManagedProviderModels } from "$lib/services/managedProviderModels";

interface ModelCache { getItem(key: string): string | null; setItem(key: string, value: string): void }
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
    const key = `vesta-dynamic-models-${family}-${apiKey.substring(0, 8)}`;
    if (!force) {
      try {
        const cached = JSON.parse(this.cache.getItem(key) || "null");
        if (Array.isArray(cached) && cached.length > 0 && cached.every(model =>
          model && typeof model.id === "string" && typeof model.name === "string")) {
          this.models = cached;
          return;
        }
      } catch { /* A malformed/unavailable cache must not prevent live discovery. */ }
    }
    this.loading = true;
    try {
      const models = await this.discover(family, apiKey);
      if (this.disposed || version !== this.version) return;
      this.models = models;
      this.cache.setItem(key, JSON.stringify(models));
    } catch (error) {
      if (!this.disposed && version === this.version) this.onError(error);
    } finally {
      if (!this.disposed && version === this.version) this.loading = false;
    }
  }
  dispose() { this.disposed = true; this.version += 1; }
}

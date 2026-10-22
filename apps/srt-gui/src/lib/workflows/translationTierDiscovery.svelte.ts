import type { DiscoveredModel } from "$lib/services/modelDiscovery";
import { credentialFingerprint } from "./credentialIdentity";

type Discover = (provider: string, apiKey: string, apiUrl: string) => Promise<DiscoveredModel[]>;
type Cache = { getItem(key: string): string | null; setItem(key: string, value: string): void };
const TTL = 24 * 60 * 60 * 1000;

/** Owns tier discovery outside reactive effects, including local endpoint polling. */
export class TranslationTierDiscovery {
  models = $state<Map<string, DiscoveredModel[]>>(new Map());
  localOffline = $state(false);
  private inflight = new Map<string, Promise<void>>();
  private generations = new Map<string, number>();
  private fingerprints = new Map<string, Map<string, string>>();
  private disposed = false;

  constructor(private readonly cache: Cache, private readonly discover: Discover) {}

  key(provider: string, keyId: string, apiKey: string, apiUrl: string) {
    let byCredential = this.fingerprints.get(keyId);
    if (!byCredential) { byCredential = new Map(); this.fingerprints.set(keyId, byCredential); }
    let token = byCredential.get(apiKey);
    if (!token) { token = `credential-${byCredential.size + 1}`; byCredential.set(apiKey, token); }
    return JSON.stringify([provider, keyId, apiUrl, token]);
  }

  async ensure(provider: string, keyId: string, apiKey: string, apiUrl: string, force = false) {
    const identity = this.key(provider, keyId, apiKey, apiUrl);
    if (this.disposed || (!force && this.models.has(identity))) return;
    if (this.inflight.has(identity)) return this.inflight.get(identity);
    const generation = (this.generations.get(identity) || 0) + 1;
    this.generations.set(identity, generation);
    let task!: Promise<void>;
    task = Promise.resolve().then(async () => {
      let cacheKey: string | null = null;
      try { cacheKey = `vesta-tier-models-v2-${await credentialFingerprint(JSON.stringify([provider, keyId, apiUrl, apiKey]))}`; }
      catch { /* cache unavailable; continue live discovery */ }
      if (!this.current(identity, generation)) return;
      if (cacheKey && !force && provider !== "local") {
        try {
          const cached = JSON.parse(this.cache.getItem(cacheKey) || "null");
          if (cached?.version === 2 && Number.isFinite(cached.ts) && Date.now() - cached.ts >= 0 && Date.now() - cached.ts < TTL && Array.isArray(cached.models) && cached.models.every((model: any) => model && typeof model.id === "string" && typeof model.name === "string")) {
            this.publish(identity, cached.models);
            return;
          }
        } catch { /* malformed storage falls through to live discovery */ }
      }
      try {
        const result = await this.discover(provider, apiKey, apiUrl);
        if (!this.current(identity, generation)) return;
        this.publish(identity, result);
        if (provider === "local") this.localOffline = false;
        if (cacheKey) try { this.cache.setItem(cacheKey, JSON.stringify({ version: 2, ts: Date.now(), models: result })); } catch { /* storage quota */ }
      } catch {
        if (this.current(identity, generation) && provider === "local") this.localOffline = true;
      }
    }).finally(() => { if (this.inflight.get(identity) === task) this.inflight.delete(identity); });
    this.inflight.set(identity, task);
    return task;
  }

  invalidate(identity: string) {
    this.generations.set(identity, (this.generations.get(identity) || 0) + 1);
    this.inflight.delete(identity);
    const next = new Map(this.models); next.delete(identity); this.models = next;
  }
  retain(identities: Set<string>) {
    for (const identity of new Set([...this.inflight.keys(), ...this.models.keys()])) {
      if (!identities.has(identity)) this.invalidate(identity);
    }
  }
  dispose() { this.disposed = true; for (const key of this.inflight.keys()) this.invalidate(key); }
  private current(identity: string, generation: number) { return !this.disposed && this.generations.get(identity) === generation; }
  private publish(identity: string, value: DiscoveredModel[]) { const next = new Map(this.models); next.set(identity, value); this.models = next; }
}

import type { DiscoveredModel } from "$lib/services/modelDiscovery";

type Discover = (apiKey: string, apiUrl: string) => Promise<DiscoveredModel[]>;
export class SpeechModelDiscovery {
  models = $state<Map<string, DiscoveredModel[]>>(new Map());
  errors = $state<Map<string, string>>(new Map());
  private generations = new Map<string, number>();
  private inflight = new Map<string, Promise<void>>();
  private disposed = false;
  constructor(private readonly discover: Discover, private readonly fallbackError: () => string) {}

  identity(keyId: string, apiKey: string, apiUrl: string) {
    let token = this.fingerprints.get(apiKey);
    if (!token) { token = `credential-${this.fingerprints.size + 1}`; this.fingerprints.set(apiKey, token); }
    return JSON.stringify([keyId, apiUrl, token]);
  }
  private fingerprints = new Map<string, string>();
  async load(identity: string, apiKey: string, apiUrl: string, force = false) {
    if (this.disposed || (!force && this.models.has(identity))) return;
    const existing = this.inflight.get(identity);
    if (existing && !force) return existing;
    const generation = (this.generations.get(identity) || 0) + 1;
    this.generations.set(identity, generation);
    let task!: Promise<void>;
    task = Promise.resolve().then(async () => {
      try {
        const result = await this.discover(apiKey, apiUrl);
        if (!this.current(identity, generation)) return;
        const models = new Map(this.models); models.set(identity, result); this.models = models;
        const errors = new Map(this.errors); errors.delete(identity); this.errors = errors;
      } catch {
        if (!this.current(identity, generation)) return;
        const errors = new Map(this.errors); errors.set(identity, this.fallbackError()); this.errors = errors;
      }
    }).finally(() => { if (this.inflight.get(identity) === task) this.inflight.delete(identity); });
    this.inflight.set(identity, task);
    return task;
  }
  invalidate(identity: string) {
    this.generations.set(identity, (this.generations.get(identity) || 0) + 1);
    this.inflight.delete(identity);
    const models = new Map(this.models); models.delete(identity); this.models = models;
    const errors = new Map(this.errors); errors.delete(identity); this.errors = errors;
  }
  retain(identities: Set<string>) {
    for (const identity of new Set([...this.inflight.keys(), ...this.models.keys(), ...this.errors.keys()])) {
      if (!identities.has(identity)) this.invalidate(identity);
    }
  }
  dispose() { this.disposed = true; for (const identity of this.inflight.keys()) this.invalidate(identity); }
  private current(identity: string, generation: number) { return !this.disposed && this.generations.get(identity) === generation; }
}

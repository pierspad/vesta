import type { DiscoveredModel } from "$lib/services/modelDiscovery";
import { LatestModelDiscovery } from "./latestModelDiscovery";

export type ProviderEndpointStatus = "idle" | "checking" | "online" | "offline";

export interface DefaultLlmSettingsSnapshot {
  provider: string;
  model: string;
  customProviderId: string;
  localServerUrl: string;
}

export interface DefaultLlmSettingsInitial extends DefaultLlmSettingsSnapshot {}

export interface EndpointStatusMessages {
  idle: string;
  checking: string;
  offline: string;
  available: (count: number) => string;
}

export class DefaultLlmProviderSettings {
  provider = $state("local");
  model = $state("");
  customProviderId = $state("");
  localServerUrl = $state("");
  discoveredModels = $state<DiscoveredModel[]>([]);
  checkingEndpoint = $state(false);
  endpointStatus = $state<ProviderEndpointStatus>("idle");
  endpointMessage = $state("");
  localProviderStatus = $state<ProviderEndpointStatus>("idle");

  private endpointDiscovery: LatestModelDiscovery;
  private localDiscovery: LatestModelDiscovery;

  constructor(
    initial: DefaultLlmSettingsInitial,
    private readonly persistSnapshot: (snapshot: DefaultLlmSettingsSnapshot) => void,
    private readonly discoverModels: (url: string, apiKey: string, timeout: number) => Promise<DiscoveredModel[]>,
  ) {
    this.provider = initial.provider;
    this.model = initial.model;
    this.customProviderId = initial.customProviderId;
    this.localServerUrl = initial.localServerUrl;
    this.endpointDiscovery = new LatestModelDiscovery(discoverModels);
    this.localDiscovery = new LatestModelDiscovery(discoverModels);
  }

  snapshot(): DefaultLlmSettingsSnapshot {
    return {
      provider: this.provider,
      model: this.model,
      customProviderId: this.customProviderId,
      localServerUrl: this.localServerUrl,
    };
  }

  persist() {
    this.persistSnapshot(this.snapshot());
  }

  selectProvider(
    provider: string,
    hasAccess: boolean,
    firstCustomProviderId: string | undefined,
    models: Array<{ id: string; recommended?: boolean }>,
  ): boolean {
    if (!hasAccess) return false;
    if (provider !== this.provider) this.resetEndpointForIdentityChange();
    this.provider = provider;
    if (provider !== "custom") this.customProviderId = "";
    else if (!this.customProviderId && firstCustomProviderId) this.customProviderId = firstCustomProviderId;

    if (provider === "local" || provider === "custom") {
      this.model = "";
    } else if (models.length > 0 && !models.some((candidate) => candidate.id === this.model)) {
      this.model = models.find((candidate) => candidate.recommended)?.id || models[0].id;
    }
    this.persist();
    return true;
  }

  selectModel(model: string) {
    this.model = model;
    this.persist();
  }

  reconcileAccess(hasAccess: boolean, firstCustomProviderId?: string) {
    if (!hasAccess) {
      if (this.provider !== "local") this.resetEndpointForIdentityChange();
      this.provider = "local";
      this.customProviderId = "";
      this.model = "";
      this.persist();
      return;
    }
    if (this.provider === "custom" && !this.customProviderId && firstCustomProviderId) {
      this.resetEndpointForIdentityChange();
      this.customProviderId = firstCustomProviderId;
      this.persist();
    }
  }

  resetLocal(url: string) {
    this.resetEndpointForIdentityChange();
    this.invalidateLocalCheck();
    this.provider = "local";
    this.localServerUrl = url;
    this.customProviderId = "";
    this.model = "";
    this.localProviderStatus = "idle";
  }

  private resetEndpointForIdentityChange() {
    this.endpointDiscovery.invalidate();
    this.checkingEndpoint = false;
    this.discoveredModels = [];
    this.endpointStatus = "idle";
    this.endpointMessage = "";
  }

  invalidateEndpointCheck() {
    this.endpointDiscovery.invalidate();
  }

  invalidateLocalCheck() {
    this.localDiscovery.invalidate();
  }

  async refreshEndpointModels(options: {
    endpointUrl: string;
    apiKey: string;
    messages: EndpointStatusMessages;
  }) {
    const { endpointUrl, apiKey, messages } = options;
    const providerAtStart = this.provider;
    const customProviderAtStart = this.customProviderId;
    const modelAtStart = this.model;
    if (this.provider !== "local" && this.provider !== "custom") {
      this.endpointDiscovery.invalidate();
      this.endpointStatus = "idle";
      this.endpointMessage = messages.idle;
      this.discoveredModels = [];
      this.checkingEndpoint = false;
      return;
    }
    if (!endpointUrl.trim()) {
      this.endpointDiscovery.invalidate();
      this.endpointStatus = "offline";
      this.endpointMessage = messages.offline;
      this.discoveredModels = [];
      this.checkingEndpoint = false;
      return;
    }
    if (this.endpointStatus === "idle") {
      this.endpointStatus = "checking";
      this.endpointMessage = messages.checking;
    }
    this.checkingEndpoint = true;
    let stale = false;
    try {
      const models = await this.endpointDiscovery.fetch(endpointUrl.trim(), apiKey, 6500);
      if (models === null || providerAtStart !== this.provider || customProviderAtStart !== this.customProviderId) { stale = true; return; }
      this.discoveredModels = models;
      this.endpointStatus = "online";
      this.endpointMessage = messages.available(models.length);
      if (this.model === modelAtStart && (!this.model || !models.some((candidate) => candidate.id === this.model))) {
        this.model = models[0]?.id || "";
        this.persist();
      }
    } catch {
      this.discoveredModels = [];
      this.endpointStatus = "offline";
      this.endpointMessage = messages.offline;
    } finally {
      if (!stale) this.checkingEndpoint = false;
    }
  }

  async refreshLocalProviderStatus(endpointUrl: string) {
    const url = endpointUrl.trim();
    if (!url) {
      this.localDiscovery.invalidate();
      this.localProviderStatus = "offline";
      return;
    }
    if (this.localProviderStatus === "idle") this.localProviderStatus = "checking";
    try {
      const models = await this.localDiscovery.fetch(url, "", 6500);
      if (models !== null) this.localProviderStatus = "online";
    } catch {
      this.localProviderStatus = "offline";
    }
  }
}

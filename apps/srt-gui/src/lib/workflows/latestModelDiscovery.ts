import { fetchModelsFromEndpoint, type DiscoveredModel } from "$lib/services/modelDiscovery";
import { LatestRequest } from "$lib/utils/asyncControl.svelte";

/** Keeps endpoint discovery results scoped to the newest check for that endpoint. */
export class LatestModelDiscovery {
  private requests = new LatestRequest();

  invalidate() {
    this.requests.invalidate();
  }

  async fetch(endpointUrl: string, apiKey: string, timeout = 6500): Promise<DiscoveredModel[] | null> {
    const request = this.requests.begin();
    try {
      const models = await fetchModelsFromEndpoint(endpointUrl, apiKey, timeout);
      return request.isCurrent() ? models : null;
    } catch (error) {
      if (!request.isCurrent()) return null;
      throw error;
    }
  }
}

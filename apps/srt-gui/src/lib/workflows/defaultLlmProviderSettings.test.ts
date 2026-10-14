import { describe, expect, it, vi } from "vitest";
import { DefaultLlmProviderSettings } from "./defaultLlmProviderSettings.svelte";

function createSettings() {
  const persist = vi.fn();
  const discover = vi.fn(async (url: string) => [{ id: `${url}-model`, name: "Endpoint model" }]);
  const settings = new DefaultLlmProviderSettings(
    { provider: "openai", model: "gpt-4o", customProviderId: "custom-1", localServerUrl: "http://localhost:11434/v1" },
    persist,
    discover,
  );
  return { settings, persist, discover };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

describe("DefaultLlmProviderSettings", () => {
  it("keeps inaccessible providers unchanged and selects valid defaults for accessible providers", () => {
    const { settings, persist } = createSettings();
    expect(settings.selectProvider("google", false, undefined, [])).toBe(false);
    expect(settings.snapshot()).toEqual({ provider: "openai", model: "gpt-4o", customProviderId: "custom-1", localServerUrl: "http://localhost:11434/v1" });
    expect(persist).not.toHaveBeenCalled();

    expect(settings.selectProvider("google", true, undefined, [
      { id: "gemini-flash" }, { id: "gemini-pro", recommended: true },
    ])).toBe(true);
    expect(settings.snapshot()).toMatchObject({ provider: "google", model: "gemini-pro", customProviderId: "" });
    settings.selectProvider("custom", true, "custom-2", []);
    expect(settings.snapshot()).toMatchObject({ provider: "custom", model: "", customProviderId: "custom-2" });
    expect(persist).toHaveBeenCalledTimes(2);
  });

  it("reconciles removed access and repairs an unset custom provider", () => {
    const { settings, persist } = createSettings();
    settings.reconcileAccess(false);
    expect(settings.snapshot()).toMatchObject({ provider: "local", model: "", customProviderId: "" });
    expect(persist).toHaveBeenCalledTimes(1);

    settings.provider = "custom";
    settings.customProviderId = "";
    settings.reconcileAccess(true, "first-custom");
    expect(settings.customProviderId).toBe("first-custom");
    expect(persist).toHaveBeenCalledTimes(2);
  });

  it("discovers endpoint models, preserves a valid model, and persists a fallback only when needed", async () => {
    const { settings, persist, discover } = createSettings();
    settings.provider = "custom";
    settings.model = "http://provider/v1-model";
    const options = {
      endpointUrl: " http://provider/v1 ", apiKey: "secret",
      messages: { idle: "idle", checking: "checking", offline: "offline", available: (count: number) => `available:${count}` },
    };
    await settings.refreshEndpointModels(options);
    expect(discover).toHaveBeenCalledWith("http://provider/v1", "secret", 6500);
    expect(settings.endpointStatus).toBe("online");
    expect(settings.endpointMessage).toBe("available:1");
    expect(settings.model).toBe("http://provider/v1-model");
    expect(persist).not.toHaveBeenCalled();

    settings.model = "removed-model";
    await settings.refreshEndpointModels({ ...options, endpointUrl: "http://provider/v1" });
    expect(settings.model).toBe("http://provider/v1-model");
    expect(persist).toHaveBeenCalledTimes(1);
  });

  it("ignores a pending custom endpoint after switching provider before effect cleanup", async () => {
    const { settings, persist, discover } = createSettings();
    settings.provider = "custom";
    const response = deferred<Awaited<ReturnType<typeof discover>>>();
    discover.mockReturnValueOnce(response.promise);
    const pending = settings.refreshEndpointModels({ endpointUrl: "http://provider/v1", apiKey: "secret", messages: { idle: "idle", checking: "checking", offline: "offline", available: () => "available" } });

    settings.selectProvider("openai", true, undefined, [{ id: "gpt-4o", recommended: true }]);
    const persistedAfterSelection = persist.mock.calls.length;
    response.resolve([{ id: "stale-model", name: "Stale" }]);
    await pending;

    expect(settings.discoveredModels).toEqual([]);
    expect(settings.endpointStatus).toBe("idle");
    expect(settings.model).toBe("gpt-4o");
    expect(persist).toHaveBeenCalledTimes(persistedAfterSelection);
  });

  it("ignores a pending custom endpoint after access reconciliation falls back to local", async () => {
    const { settings, persist, discover } = createSettings();
    settings.provider = "custom";
    const response = deferred<Awaited<ReturnType<typeof discover>>>();
    discover.mockReturnValueOnce(response.promise);
    const pending = settings.refreshEndpointModels({ endpointUrl: "http://provider/v1", apiKey: "secret", messages: { idle: "idle", checking: "checking", offline: "offline", available: () => "available" } });

    settings.reconcileAccess(false);
    const persistedAfterFallback = persist.mock.calls.length;
    response.resolve([{ id: "stale-model", name: "Stale" }]);
    await pending;

    expect(settings.snapshot()).toMatchObject({ provider: "local", model: "", customProviderId: "" });
    expect(settings.discoveredModels).toEqual([]);
    expect(persist).toHaveBeenCalledTimes(persistedAfterFallback);
  });

  it("does not replace a model chosen while endpoint discovery is pending", async () => {
    const { settings, persist, discover } = createSettings();
    settings.provider = "custom";
    settings.model = "";
    const response = deferred<Awaited<ReturnType<typeof discover>>>();
    discover.mockReturnValueOnce(response.promise);
    const pending = settings.refreshEndpointModels({ endpointUrl: "http://provider/v1", apiKey: "secret", messages: { idle: "idle", checking: "checking", offline: "offline", available: () => "available" } });

    settings.selectModel("chosen-by-user");
    response.resolve([{ id: "discovered-default", name: "Discovered" }]);
    await pending;

    expect(settings.model).toBe("chosen-by-user");
    expect(persist).toHaveBeenCalledTimes(1);
  });
});

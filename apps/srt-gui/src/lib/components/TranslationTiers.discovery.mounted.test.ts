// @vitest-environment happy-dom
import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const setup = vi.hoisted(() => ({
  entries: [{ id: "entry", provider: "local", apiKeyId: "local", model: "" }],
  discover: vi.fn(),
  saveCache: vi.fn(),
}));
vi.mock("$lib/config/translationTiers", () => ({
  loadTiers: () => [{ id: "tier", entries: setup.entries }], saveTiers: vi.fn(), newTierId: () => "tier-new",
  newTierEntryId: () => "entry-new", TIERS_UPDATED_EVENT: "tiers-updated",
}));
vi.mock("$lib/config/apiKeys", () => ({
  loadAndValidateApiKeys: () => [{ id: "local", name: "Local", apiType: "local", apiKey: "", apiUrl: "http://localhost:1234" }],
  supportsCapability: () => true,
}));
vi.mock("$lib/config/llmProviders", () => ({ providers: { local: { name: "Local", defaultApiUrl: "http://localhost:1234" } }, getModelsForProvider: () => [] }));
vi.mock("$lib/services/modelDiscovery", () => ({ discoverModels: setup.discover }));
vi.mock("$lib/config/vestaConfig", () => ({ getItem: () => null, setItem: setup.saveCache }));
import TranslationTiers from "./TranslationTiers.svelte";

let component: ReturnType<typeof mount> | undefined;
beforeEach(() => vi.clearAllMocks());
afterEach(async () => { if (component) await unmount(component); component = undefined; vi.useRealTimers(); document.body.replaceChildren(); });
async function render() { component = mount(TranslationTiers, { target: document.body }); await tick(); }

it("does not rediscover a successful local result before the polling interval", async () => {
  vi.useFakeTimers(); setup.discover.mockResolvedValue([{ id: "local-model", name: "Local model" }]);
  await render(); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledOnce());
  await vi.advanceTimersByTimeAsync(14_999); expect(setup.discover).toHaveBeenCalledOnce();
  await vi.advanceTimersByTimeAsync(1); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledTimes(2));
});

it("does not immediately retry an empty result and clears its timer on teardown", async () => {
  vi.useFakeTimers(); setup.discover.mockResolvedValue([]);
  await render(); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledOnce());
  await vi.advanceTimersByTimeAsync(14_999); expect(setup.discover).toHaveBeenCalledOnce();
  await unmount(component!); component = undefined;
  await vi.advanceTimersByTimeAsync(30_000); expect(setup.discover).toHaveBeenCalledOnce();
});

it("waits for the polling interval before retrying a failed local endpoint", async () => {
  vi.useFakeTimers(); setup.discover.mockRejectedValue(new Error("offline"));
  await render(); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledOnce());
  await vi.advanceTimersByTimeAsync(14_999); expect(setup.discover).toHaveBeenCalledOnce();
  setup.discover.mockResolvedValue([]);
  await vi.advanceTimersByTimeAsync(1); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledTimes(2));
});

it("does not publish a pending local response after component teardown", async () => {
  let resolve!: (models: { id: string; name: string }[]) => void;
  setup.discover.mockImplementation(() => new Promise(done => resolve = done));
  await render(); await vi.waitFor(() => expect(setup.discover).toHaveBeenCalledOnce());
  await unmount(component!); component = undefined;
  resolve([{ id: "late", name: "Late model" }]); await tick();
  expect(setup.saveCache).not.toHaveBeenCalled();
  expect(document.body.textContent).not.toContain("Late model");
});

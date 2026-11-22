import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchModelsFromEndpoint } from "$lib/services/modelDiscovery";
import { LatestModelDiscovery } from "./latestModelDiscovery";

vi.mock("$lib/services/modelDiscovery", () => ({ fetchModelsFromEndpoint: vi.fn() }));
const mockedFetch = vi.mocked(fetchModelsFromEndpoint);

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

describe("LatestModelDiscovery", () => {
  beforeEach(() => vi.resetAllMocks());

  it("returns only the newest endpoint result when checks overlap", async () => {
    const oldResponse = deferred<Awaited<ReturnType<typeof fetchModelsFromEndpoint>>>();
    const newResponse = deferred<Awaited<ReturnType<typeof fetchModelsFromEndpoint>>>();
    mockedFetch.mockReturnValueOnce(oldResponse.promise).mockReturnValueOnce(newResponse.promise);
    const discovery = new LatestModelDiscovery();

    const oldCheck = discovery.fetch("http://old/v1", "old-key");
    const newCheck = discovery.fetch("http://new/v1", "new-key");
    newResponse.resolve([{ id: "new-model", name: "New model" }]);
    oldResponse.resolve([{ id: "old-model", name: "Old model" }]);

    await expect(newCheck).resolves.toEqual([{ id: "new-model", name: "New model" }]);
    await expect(oldCheck).resolves.toBeNull();
    expect(mockedFetch).toHaveBeenNthCalledWith(1, "http://old/v1", "old-key", 6500);
    expect(mockedFetch).toHaveBeenNthCalledWith(2, "http://new/v1", "new-key", 6500);
  });

  it("ignores a successful result after the owning effect is cleaned up", async () => {
    const response = deferred<Awaited<ReturnType<typeof fetchModelsFromEndpoint>>>();
    mockedFetch.mockReturnValueOnce(response.promise);
    const discovery = new LatestModelDiscovery();
    const check = discovery.fetch("http://localhost:11434/v1", "");

    discovery.invalidate();
    response.resolve([{ id: "stale-model", name: "Stale model" }]);

    await expect(check).resolves.toBeNull();
  });

  it("ignores an old error after the same endpoint is checked again", async () => {
    const closedCheck = deferred<Awaited<ReturnType<typeof fetchModelsFromEndpoint>>>();
    const reopenedCheck = deferred<Awaited<ReturnType<typeof fetchModelsFromEndpoint>>>();
    mockedFetch.mockReturnValueOnce(closedCheck.promise).mockReturnValueOnce(reopenedCheck.promise);
    const discovery = new LatestModelDiscovery();

    const oldCheck = discovery.fetch("http://localhost:11434/v1", "key");
    discovery.invalidate();
    const newCheck = discovery.fetch("http://localhost:11434/v1", "key");

    closedCheck.reject(new Error("old connection closed"));
    await expect(oldCheck).resolves.toBeNull();
    reopenedCheck.resolve([{ id: "current-model", name: "Current model" }]);
    await expect(newCheck).resolves.toEqual([{ id: "current-model", name: "Current model" }]);
  });

  it("surfaces errors from the current request", async () => {
    mockedFetch.mockRejectedValueOnce(new Error("offline"));
    await expect(new LatestModelDiscovery().fetch("http://localhost:11434/v1", ""))
      .rejects.toThrow("offline");
  });
});

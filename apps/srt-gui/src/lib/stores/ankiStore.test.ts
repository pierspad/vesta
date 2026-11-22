// @vitest-environment happy-dom
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  store: new Map<string, string>(),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => mocks.store.get(key) ?? null,
  setItem: (key: string, value: string) => {
    mocks.store.set(key, value);
  },
}));

import { ankiStore } from "./ankiStore.svelte";

describe("ankiStore", () => {
  beforeEach(async () => {
    mocks.invoke.mockReset();
    mocks.store.clear();
    mocks.invoke.mockRejectedValue(new Error("unconfigured"));
    ankiStore.reinit();
    await Promise.resolve();
    mocks.invoke.mockReset();
  });

  it("checks AnkiConnect connectivity, loads decks, and handles deck-fetch failures gracefully", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "ankiconnect_ping") return 6;
      if (cmd === "ankiconnect_deck_names") return ["Default", "Japanese::Anime"];
      throw new Error(`Unexpected command: ${cmd}`);
    });

    const ok = await ankiStore.checkConnection(false);
    expect(ok).toBe(true);
    expect(ankiStore.status).toBe("online");
    expect(ankiStore.version).toBe(6);
    expect(ankiStore.decks).toEqual(["Default", "Japanese::Anime"]);

    // Ping succeeds even if fetching deck names fails afterwards
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "ankiconnect_ping") return 6;
      if (cmd === "ankiconnect_deck_names") throw new Error("timeout");
      throw new Error(`Unexpected command: ${cmd}`);
    });

    const partialOk = await ankiStore.checkConnection(true);
    expect(partialOk).toBe(true);
    expect(ankiStore.status).toBe("online");

    // Ping fails -> offline
    mocks.invoke.mockRejectedValueOnce(new Error("connection refused"));
    const failed = await ankiStore.checkConnection(false);
    expect(failed).toBe(false);
    expect(ankiStore.status).toBe("offline");
  });

  it("persists URL and Anki card preferences and restores them on reinit", async () => {
    mocks.invoke.mockResolvedValue(6);

    ankiStore.setUrl("http://127.0.0.1:9999");
    ankiStore.setDictionaryLookup(false);
    ankiStore.setAutoCardFont(true);
    ankiStore.setEmbedCardFont(true);

    expect(mocks.store.get("vesta-ankiconnect-url")).toBe("http://127.0.0.1:9999");
    expect(mocks.store.get("vesta-dictionary-lookup-enabled")).toBe("false");
    expect(mocks.store.get("vesta-auto-card-font")).toBe("true");
    expect(mocks.store.get("vesta-embed-card-font")).toBe("true");

    ankiStore.reinit();
    await Promise.resolve();
    expect(ankiStore.url).toBe("http://127.0.0.1:9999");
    expect(ankiStore.dictionaryLookup).toBe(false);
    expect(ankiStore.autoCardFont).toBe(true);
    expect(ankiStore.embedCardFont).toBe(true);
  });
});

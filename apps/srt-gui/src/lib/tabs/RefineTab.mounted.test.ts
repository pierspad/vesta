// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount, tick, unmount } from "svelte";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  open: vi.fn(),
  save: vi.fn(),
  store: new Map<string, string>(),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));
vi.mock("$lib/utils/dialogGuard", () => ({
  guardedOpen: mocks.open,
  guardedSave: mocks.save,
}));
vi.mock("$lib/utils/dragDrop", () => ({
  setupWebviewDragDrop: vi.fn(() => () => {}),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => mocks.store.get(key) ?? null,
  setItem: (key: string, value: string) => {
    mocks.store.set(key, value);
  },
  removeItem: (key: string) => {
    mocks.store.delete(key);
  },
  initConfig: vi.fn().mockResolvedValue(undefined),
  flush: vi.fn().mockResolvedValue(undefined),
}));

import RefineTab from "./RefineTab.svelte";

const mounted: ReturnType<typeof mount>[] = [];

async function settle(turns = 8) {
  for (let i = 0; i < turns; i++) {
    await Promise.resolve();
    await tick();
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  mocks.store.clear();
  mocks.store.set(
    "vesta-api-keys",
    JSON.stringify([
      {
        id: "key-google",
        name: "Gemini",
        apiType: "google",
        apiKey: "AIzaTestKey1234567890",
        capabilities: ["llm"],
      },
    ]),
  );
  mocks.store.set(
    "vesta-translate-tiers",
    JSON.stringify([
      {
        id: "tier-1",
        entries: [{ id: "e1", provider: "google", apiKeyId: "key-google", model: "gemini-2.5-flash" }],
      },
    ]),
  );
});

afterEach(async () => {
  for (const component of mounted.splice(0)) {
    await unmount(component);
  }
  document.body.replaceChildren();
});

describe("RefineTab", () => {
  it("loads deck cards, filters by search query, annotates a card with AI, and switches to Auto mode", async () => {
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "refine_load_file") {
        return [
          { id: "n1", expression: "食べる", meaning: "to eat", notes: "" },
          { id: "n2", expression: "飲む", meaning: "to drink", notes: "Existing note" },
        ];
      }
      if (cmd === "refine_card_llm_tiered") {
        return "Ichidan verb meaning to consume food.";
      }
      if (cmd === "refine_save_file") {
        return true;
      }
      return null;
    });

    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(RefineTab, { target, props: { active: true } }));
    await settle();

    await (window as any).loadRefineFile("/decks/sample.apkg");
    await settle();

    expect(mocks.invoke).toHaveBeenCalledWith("refine_load_file", { path: "/decks/sample.apkg" });
    expect(target.textContent).toContain("食べる");
    expect(target.textContent).toContain("飲む");

    // Search filter narrows down cards
    const searchInput = target.querySelector<HTMLInputElement>('input[placeholder*="Search flashcards"]')!;
    expect(searchInput).not.toBeNull();
    searchInput.value = "飲む";
    searchInput.dispatchEvent(new Event("input", { bubbles: true }));
    await settle();

    expect(target.textContent).toContain("飲む");

    // Annotate selected card via AI
    const aiBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Annotate with AI"),
    )!;
    expect(aiBtn).toBeDefined();
    expect(aiBtn.disabled).toBe(false);
    aiBtn.click();
    await settle();

    expect(mocks.invoke).toHaveBeenCalledWith(
      "refine_card_llm_tiered",
      expect.objectContaining({
        card: expect.objectContaining({ id: "n2" }),
      }),
    );
    expect(target.querySelector<HTMLTextAreaElement>("#card-notes")?.value).toBe(
      "Ichidan verb meaning to consume food.",
    );

    // Switch to Auto mode panel
    const autoBtn = Array.from(target.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
      b.textContent?.includes("Automatic (AI)"),
    )!;
    expect(autoBtn).toBeDefined();
    autoBtn.click();
    await settle();
    expect(autoBtn.getAttribute("aria-pressed")).toBe("true");
  });
});

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { storage, mocks } = vi.hoisted(() => ({
  storage: new Map<string, string>(),
  mocks: {
    invoke: vi.fn(),
  },
}));

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
}));

vi.mock("$lib/services/tauriClient", () => ({
  invokeCommand: mocks.invoke,
}));

import { cpuRamStore } from "./cpuRamStore.svelte";

describe("cpuRamStore", () => {
  beforeEach(() => {
    storage.clear();
    vi.clearAllMocks();
    vi.stubGlobal("window", { dispatchEvent: vi.fn() });
    vi.stubGlobal(
      "CustomEvent",
      class {
        constructor(
          public type: string,
          public options?: unknown,
        ) {}
      },
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("initializes bounds from backend and clamps stored cores and RAM limit", async () => {
    storage.set("vesta_cpu_cores", "64");
    storage.set("vesta_memory_limit_mb", "99999");
    mocks.invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "flashcard_get_cpu_count") return 8;
      if (cmd === "flashcard_get_total_memory_mb") return 8192;
      return 0;
    });
    cpuRamStore.ramLimitMb = 99999;

    await cpuRamStore.init();

    expect(cpuRamStore.systemCpuCount).toBe(8);
    expect(cpuRamStore.maxCpuCores).toBe(7);
    expect(cpuRamStore.cpuCores).toBe(7);
    expect(cpuRamStore.ramLimitMb).toBe(7680);
  });

  it("sets cores with event broadcast and removes memory limit key when cleared", () => {
    cpuRamStore.setCores(3);
    expect(storage.get("vesta_cpu_cores")).toBe("3");
    expect(window.dispatchEvent).toHaveBeenCalled();

    cpuRamStore.setRamLimitMb(2048);
    expect(storage.get("vesta_memory_limit_mb")).toBe("2048");

    cpuRamStore.setRamLimitMb(100);
    expect(cpuRamStore.ramLimitMb).toBe(0);
    expect(storage.has("vesta_memory_limit_mb")).toBe(false);
  });
});

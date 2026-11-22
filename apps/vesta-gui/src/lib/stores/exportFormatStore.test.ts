import { describe, it, expect, beforeEach, vi } from "vitest";

const { storage } = vi.hoisted(() => {
  return { storage: new Map<string, string>() };
});

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => {
    storage.set(key, val);
  },
  removeItem: (key: string) => {
    storage.delete(key);
  },
}));

import { exportFormatStore } from "./exportFormatStore.svelte";
import { ankiStore } from "$lib/stores/ankiStore.svelte";

describe("exportFormatStore", () => {
  beforeEach(() => {
    storage.clear();
    exportFormatStore.setExportFormat("apkg");
    exportFormatStore.setFallbackFormat("apkg");
    ankiStore.status = "offline";
  });

  it("cycles through available export formats in order", () => {
    expect(exportFormatStore.exportFormat).toBe("apkg");

    exportFormatStore.cycleExportFormat();
    expect(exportFormatStore.exportFormat).toBe("tsv");

    exportFormatStore.cycleExportFormat();
    expect(exportFormatStore.exportFormat).toBe("anki");

    exportFormatStore.cycleExportFormat();
    expect(exportFormatStore.exportFormat).toBe("apkg");
  });

  it("resolves effectiveExportFormat seamlessly when anki is offline", () => {
    exportFormatStore.setExportFormat("anki");
    exportFormatStore.setFallbackFormat("tsv");
    ankiStore.status = "offline";

    // Should fall back to tsv because Anki is offline
    expect(exportFormatStore.effectiveExportFormat).toBe("tsv");

    // When Anki connects online, effective becomes anki
    ankiStore.status = "online";
    expect(exportFormatStore.effectiveExportFormat).toBe("anki");
  });

  it("persists changes in config storage", () => {
    exportFormatStore.setExportFormat("tsv");
    expect(storage.get("vesta-export-format")).toBe("tsv");

    exportFormatStore.setFallbackFormat("apkg");
    expect(storage.get("vesta-export-fallback")).toBe("apkg");
  });
});

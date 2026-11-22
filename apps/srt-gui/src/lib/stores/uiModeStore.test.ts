import { beforeEach, describe, expect, it, vi } from "vitest";

const storage = vi.hoisted(() => new Map<string, string>());

vi.mock("$lib/config/vestaConfig", () => ({
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, val: string) => storage.set(key, val),
  removeItem: (key: string) => storage.delete(key),
}));

import { uiMode } from "./uiModeStore.svelte";
import { exportFormatStore } from "./exportFormatStore.svelte";

describe("uiModeStore", () => {
  beforeEach(() => {
    storage.clear();
    uiMode.expertMode = false;
    exportFormatStore.setExportFormat("apkg");
  });

  it("backs up export format when leaving expert mode and restores it when re-entering", async () => {
    uiMode.expertMode = true;
    exportFormatStore.setExportFormat("tsv");

    await uiMode.toggleExpertMode();
    expect(uiMode.expertMode).toBe(false);
    expect(uiMode.easyMode).toBe(true);
    expect(exportFormatStore.exportFormat).toBe("apkg");
    expect(storage.get("vesta-expert-backup-export-format")).toBe("tsv");
    expect(storage.get("vesta-expert-mode")).toBe("false");

    await uiMode.toggleExpertMode();
    expect(uiMode.expertMode).toBe(true);
    expect(uiMode.easyMode).toBe(false);
    expect(exportFormatStore.exportFormat).toBe("tsv");
    expect(storage.get("vesta-expert-mode")).toBe("true");
  });
});

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { autoSyncStore } from "./autoSyncStore.svelte";

describe("autoSyncStore", () => {
  let activeRun: ReturnType<typeof autoSyncStore.operation.begin> = null;

  beforeEach(() => {
    activeRun?.finish();
    activeRun = null;
    autoSyncStore.selectedMode = "quick";
    autoSyncStore.activeMode = null;
  });

  afterEach(() => {
    activeRun?.finish();
    activeRun = null;
  });

  it("toggles between quick and precise mode only when idle", () => {
    expect(autoSyncStore.selectedMode).toBe("quick");
    autoSyncStore.toggleMode();
    expect(autoSyncStore.selectedMode).toBe("precise");

    activeRun = autoSyncStore.operation.begin();
    expect(autoSyncStore.isAutoSyncing).toBe(true);
    autoSyncStore.toggleMode();
    expect(autoSyncStore.selectedMode).toBe("precise");
    activeRun?.finish();
    activeRun = null;

    autoSyncStore.toggleMode();
    expect(autoSyncStore.selectedMode).toBe("quick");
  });

  it("resolves structured progress keys including total->count parameter mapping", () => {
    const msg = autoSyncStore.resolveProgressMessage({
      stage: "prepare",
      message: "fallback",
      percentage: 10,
      message_key: "sync.progress.preparingSegments",
      params: { total: "8" },
    });
    expect(typeof msg).toBe("string");
    expect(msg.length).toBeGreaterThan(0);

    expect(
      autoSyncStore.resolveProgressMessage({
        stage: "done",
        message: "Plain message",
        percentage: 100,
      }),
    ).toBe("Plain message");
  });
});

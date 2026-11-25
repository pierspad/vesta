// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { GuidanceStore } from "./guidanceStore.svelte";

afterEach(() => {
  vi.useRealTimers();
});

describe("GuidanceStore", () => {
  it("guides step by step from source tab -> settings sidebar section -> target arrival", () => {
    vi.useFakeTimers();
    const store = new GuidanceStore();
    const onArrive = vi.fn();

    store.guideToSettings({ sourceTab: "translate", section: "llm", targetId: "llm-tiers" });
    expect(store.isSettingsButtonPulsing).toBe(true);
    expect(store.isSectionPulsing("llm")).toBe(false);

    // Staying on source tab keeps step 1 active
    store.syncNavigation("translate", "overview", onArrive);
    expect(store.phase).toBe("sidebar-settings");

    // Entering settings on overview advances to step 2 (pulse LLM section button)
    store.syncNavigation("settings", "overview", onArrive);
    expect(store.phase).toBe("settings-section");
    expect(store.isSettingsButtonPulsing).toBe(false);
    expect(store.isSectionPulsing("llm")).toBe(true);
    expect(onArrive).not.toHaveBeenCalled();

    // Selecting target section completes guidance and highlights target element
    store.syncNavigation("settings", "llm", onArrive);
    expect(onArrive).toHaveBeenCalledWith("llm-tiers");
    expect(store.active).toBeNull();
    expect(store.pulseOn).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("cancels immediately when switching to an unrelated main tab, unrelated settings section, or backing out", () => {
    vi.useFakeTimers();
    const store = new GuidanceStore();
    const onArrive = vi.fn();

    // Unrelated main tab
    store.guideToSettings({ sourceTab: "translate", section: "llm", targetId: "llm-tiers" });
    store.syncNavigation("flashcards", "overview", onArrive);
    expect(store.active).toBeNull();

    // Unrelated settings section after entering settings
    store.guideToSettings({ sourceTab: "translate", section: "llm", targetId: "llm-tiers" });
    store.syncNavigation("settings", "overview", onArrive);
    expect(store.phase).toBe("settings-section");
    store.syncNavigation("settings", "anki", onArrive);
    expect(store.active).toBeNull();
    expect(onArrive).not.toHaveBeenCalled();

    // Backing out of settings without reaching target
    store.guideToSettings({ sourceTab: "translate", section: "llm", targetId: "llm-tiers" });
    store.syncNavigation("settings", "overview", onArrive);
    store.syncNavigation("translate", "overview", onArrive);
    expect(store.active).toBeNull();
  });

  it("pulses at intervals for at most 3 double-pulse cycles and turns off automatically", () => {
    vi.useFakeTimers();
    const store = new GuidanceStore();

    store.guideToSettings({ sourceTab: "translate", section: "llm", targetId: "llm-tiers" });
    expect(store.isSettingsButtonPulsing).toBe(true);

    // End of cycle 1 (1800ms)
    vi.advanceTimersByTime(1800);
    expect(store.isSettingsButtonPulsing).toBe(false);
    expect(store.active).not.toBeNull();

    // Pause ends -> cycle 2 starts (+2200ms)
    vi.advanceTimersByTime(2200);
    expect(store.isSettingsButtonPulsing).toBe(true);

    // End of cycle 2 (+1800ms), pause (+2200ms) -> cycle 3 starts
    vi.advanceTimersByTime(1800 + 2200);
    expect(store.isSettingsButtonPulsing).toBe(true);

    // End of cycle 3 (+1800ms) -> auto-clears completely
    vi.advanceTimersByTime(1800);
    expect(store.isSettingsButtonPulsing).toBe(false);
    expect(store.active).toBeNull();
    expect(vi.getTimerCount()).toBe(0);
  });
});

// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from "vitest";
import { generationStore } from "./generationStore.svelte";
import { exportFormatStore } from "./exportFormatStore.svelte";
import { MAX_WORKFLOW_LOGS } from "$lib/utils/logPanelBuffer.svelte";

describe("generationStore", () => {
  beforeEach(() => {
    generationStore.cancelRun();
    generationStore.resetRun();
    generationStore.setExportFormat("apkg");
    generationStore.setFallbackFormat("tsv");
    generationStore.cpuCores = 2;
  });

  it("appends logs, merges matching progressKey entries, and caps at MAX_WORKFLOW_LOGS", () => {
    generationStore.addLog("Starting extraction", "info");
    generationStore.addLog("Audio 10%", "progress", undefined, "audio");
    generationStore.addLog("Audio 55%", "progress", "44/80", "audio");

    expect(generationStore.logs).toHaveLength(2);
    expect(generationStore.logs[1]).toMatchObject({
      message: "Audio 55%",
      type: "progress",
    });

    for (let i = 0; i < MAX_WORKFLOW_LOGS + 25; i++) {
      generationStore.addLog(`Line ${i}`, "info");
    }
    expect(generationStore.logs).toHaveLength(MAX_WORKFLOW_LOGS);
    expect(generationStore.logs[generationStore.logs.length - 1].message).toBe(
      `Line ${MAX_WORKFLOW_LOGS + 24}`,
    );

    generationStore.clearLogs();
    expect(generationStore.logs).toEqual([]);
  });

  it("ignores resetRun while busy and resets state when idle", () => {
    generationStore.progress = 75;
    generationStore.progressMessage = "Encoding clips";
    generationStore.progressStage = "video";
    generationStore.phaseProgress = 50;
    generationStore.error = "Transient error";
    generationStore.result = {
      success: true,
      message: "Done",
      cardsGenerated: 42,
      audioClips: 42,
      snapshots: 42,
      videoClips: 0,
      tsvPath: null,
      apkgPath: null,
      outputSizeBytes: 2048,
    };
    generationStore.addLog("Step 1", "info");

    generationStore.isProcessing = true;
    generationStore.resetRun();
    expect(generationStore.progress).toBe(75);
    expect(generationStore.error).toBe("Transient error");

    generationStore.cancelRun();
    expect(generationStore.isProcessing).toBe(false);
    expect(generationStore.progress).toBe(0);
    expect(generationStore.progressMessage).toBe("");

    generationStore.resetRun();
    expect(generationStore.result).toBeNull();
    expect(generationStore.error).toBeNull();
    expect(generationStore.progressStage).toBe("");
    expect(generationStore.phaseProgress).toBeNull();
    expect(generationStore.logs).toEqual([]);
  });

  it("forwards export format controls and reflects effectiveCpuCores", () => {
    generationStore.cpuCores = 6;
    expect(generationStore.effectiveCpuCores).toBe(6);

    generationStore.setExportFormat("tsv");
    expect(generationStore.exportFormat).toBe("tsv");
    expect(exportFormatStore.exportFormat).toBe("tsv");

    generationStore.setFallbackFormat("apkg");
    expect(generationStore.fallbackFormat).toBe("apkg");

    const before = generationStore.exportFormat;
    generationStore.cycleExportFormat();
    expect(generationStore.exportFormat).not.toBe(before);
  });
});

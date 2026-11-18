import { expect, it, vi } from "vitest";
import { createLogPanelBuffer, MAX_WORKFLOW_LOGS } from "./logPanelBuffer.svelte";
import { generationStore } from "$lib/stores/generationStore.svelte";

vi.mock("$lib/config/vestaConfig", () => ({ getItem: () => null }));

it("bounds workflow logs and preserves recent entries", () => {
  const buffer = createLogPanelBuffer();
  for (let i = 0; i < MAX_WORKFLOW_LOGS + 20; i++) buffer.addLog(String(i));
  expect(buffer.logs).toHaveLength(MAX_WORKFLOW_LOGS);
  expect(buffer.logs[0].message).toBe("20");
  expect(new Set(buffer.logs.map(log => log.id)).size).toBe(MAX_WORKFLOW_LOGS);
  buffer.clearLogs();
  expect(buffer.logs).toHaveLength(0);
});

it("bounds flashcard logs and coalesces only consecutive progress with the same key", () => {
  generationStore.resetRun();
  for (let i = 0; i < MAX_WORKFLOW_LOGS + 20; i++) generationStore.addLog(String(i));
  expect(generationStore.logs).toHaveLength(MAX_WORKFLOW_LOGS);
  expect(generationStore.logs[0].message).toBe("20");
  generationStore.addLog("first", "progress", undefined, "audio");
  const id = generationStore.logs.at(-1)?.id;
  generationStore.addLog("updated", "progress", undefined, "audio");
  expect(generationStore.logs.at(-1)?.id).toBe(id);
  generationStore.addLog("unkeyed", "progress");
  generationStore.addLog("next", "progress", undefined, "audio");
  expect(generationStore.logs.slice(-3).map(log => log.message)).toEqual(["updated", "unkeyed", "next"]);
  expect(generationStore.logs).toHaveLength(MAX_WORKFLOW_LOGS);
  generationStore.resetRun();
});

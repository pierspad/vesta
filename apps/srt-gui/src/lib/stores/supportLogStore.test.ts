import { expect, it, vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { supportLogStore as store } from "./supportLogStore.svelte";
import { recordSupportEvent } from "$lib/utils/supportLog";
it("flushes the complete log snapshot before export and persists before stop", async () => {
  vi.mocked(invoke).mockImplementation(async command => command === "support_log_start" ? "/tmp/session.jsonl" : command === "support_log_export" ? "/tmp/export.jsonl" : undefined);
  await store.start(); expect(store.recording).toBe(true);
  for (let i = 0; i < 250; i++) recordSupportEvent("test", `entry-${i}`);
  expect(store.preview).toHaveLength(200);
  expect(store.preview[0].message).toBe("entry-50");
  expect(store.previewText).toContain("entry-249");
  await store.export("/tmp/export.jsonl");
  expect(store.preview).toHaveLength(200);
  const exportedCalls = vi.mocked(invoke).mock.calls;
  const exportIndex = exportedCalls.findIndex(([command]) => command === "support_log_export");
  const batches = exportedCalls.slice(0, exportIndex).filter(([command]) => command === "support_log_append");
  expect(batches.flatMap(([, args]) => (args as { entries: string[] })?.entries)).toHaveLength(251);
  recordSupportEvent("test", "last entry"); await store.stop();
  expect(store.recording).toBe(false);
  expect(invoke).toHaveBeenLastCalledWith("support_log_stop");
  expect(store.path).toBe("/tmp/session.jsonl");
});

it("records retry metadata without copying unexpected provider payload fields", async () => {
  vi.mocked(invoke).mockClear();
  vi.mocked(invoke).mockImplementation(async command => command === "support_log_start" ? "/tmp/session.jsonl" : undefined);
  vi.mocked(listen).mockImplementation(async (name, callback) => {
    if (name === "provider-retry") callback({ event: name, id: 0, payload: { tier: 1, endpoint: 2, status: 429, attempt: 1, max_retries: 3, retry_in_ms: 3000, body: "PRIVATE-SUBTITLE", api_key: "private-key" } } as never);
    return () => {};
  });
  await store.start();
  await store.stop();
  const entries = vi.mocked(invoke).mock.calls.filter(([command]) => command === "support_log_append")
    .flatMap(([, args]) => (args as { entries: string[] }).entries).map(entry => JSON.parse(entry));
  const retry = entries.find(entry => entry.kind === "provider-retry");
  expect(JSON.parse(retry.message)).toEqual({ tier: 1, endpoint: 2, status: 429, attempt: 1, max_retries: 3, retry_in_ms: 3000 });
  expect(JSON.stringify(entries)).not.toMatch(/PRIVATE-SUBTITLE|private-key/);
  expect(store.previewText).not.toMatch(/PRIVATE-SUBTITLE|private-key/);
  expect(store.previewText).toContain("Recording stopped");
});

import { expect, it, vi } from "vitest";
import { invokeCommand } from "$lib/services/tauriClient";
import { diagnosticsStore } from "./diagnosticsStore.svelte";
import type { SystemDiagnostics } from "$lib/services/systemDiagnostics";
it("deduplicates pending checks, caches reopening, and permits a forced refresh", async () => {
  const data = { os: "linux", arch: "x86_64" } as SystemDiagnostics;
  let resolve!: (value: SystemDiagnostics) => void;
  vi.mocked(invokeCommand).mockReset().mockImplementation(() => new Promise(done => { resolve = done; }));
  const pending = diagnosticsStore.refresh();
  expect(diagnosticsStore.refresh()).toBe(pending);
  expect(invokeCommand).toHaveBeenCalledOnce();
  resolve(data); await pending;
  await diagnosticsStore.refresh(); expect(invokeCommand).toHaveBeenCalledOnce();
  vi.mocked(invokeCommand).mockResolvedValue(data);
  await diagnosticsStore.refresh(true); expect(invokeCommand).toHaveBeenCalledTimes(2);
  expect(diagnosticsStore.data).toEqual(data); expect(diagnosticsStore.loading).toBe(false);
});

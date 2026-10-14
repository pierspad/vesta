import { afterEach, expect, it, vi } from "vitest";
vi.unmock("$lib/services/tauriClient");
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
import { invokeCommand } from "./tauriClient";
import { setSupportLogSink } from "$lib/utils/supportLog";

afterEach(() => { setSupportLogSink(null); vi.clearAllMocks(); });

it("returns successful IPC results even when diagnostic recording fails", async () => {
  setSupportLogSink(() => { throw new Error("logger failure"); });
  vi.mocked(invoke).mockResolvedValue({ ok: true });
  expect(await invokeCommand("example")).toEqual({ ok: true });
});

it("preserves the original IPC rejection identity", async () => {
  const failure = { message: "disk full", code: "IO" };
  vi.mocked(invoke).mockRejectedValue(failure);
  setSupportLogSink(() => { throw new Error("logger failure"); });
  await expect(invokeCommand("example")).rejects.toBe(failure);
});

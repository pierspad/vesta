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

it("logs a failed workflow result without dumping its data", async () => {
  const sink = vi.fn();
  setSupportLogSink(sink);
  const result = { success: false, message: "FFmpeg failed", content: "private text" };
  vi.mocked(invoke).mockResolvedValue(result);
  expect(await invokeCommand("generate_flashcards")).toBe(result);
  expect(sink.mock.calls.map(([entry]) => entry.kind)).toEqual(["ipc.start", "ipc.failed"]);
  expect(sink.mock.lastCall?.[0].message).toMatch(/^generate_flashcards \(\d+ ms\): FFmpeg failed$/);
});

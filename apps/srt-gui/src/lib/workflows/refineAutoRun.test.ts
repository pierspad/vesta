import { describe, expect, it, vi } from "vitest";
import { runRefineAuto, type RefineListen, type RefineRunSummary } from "./refineAutoRun";

const summary: RefineRunSummary = { done: 1, failed: 0, remaining: 0, poolExhausted: false, cancelled: false };
const cards = [{ id: "card-1", expression: "hola", meaning: "hello", notes: "" }];

describe("runRefineAuto", () => {
  it("registers before invoking and forwards only progress for its run ID", async () => {
    const order: string[] = [];
    let handler: ((event: any) => void) | undefined;
    const progress = vi.fn();
    const cleanup = vi.fn();
    let finishInvocation!: (value: RefineRunSummary) => void;
    const subscribe = vi.fn(async (_name: string, callback: (event: any) => void) => {
      order.push("listen");
      handler = callback;
      return cleanup;
    }) as unknown as RefineListen;
    const invoke = vi.fn(() => {
      order.push("invoke");
      return new Promise<RefineRunSummary>(resolve => { finishInvocation = resolve; });
    });

    const pending = runRefineAuto({
      cards, prompt: "prompt", tiers: [[{ provider: "openai", model: "gpt", api_key: null, api_url: null, rpm: null, max_requests: null }]], batchMode: true,
      isCancelled: () => false, onProgress: progress, subscribe, invoke, createRunId: () => "run-current",
    });
    await Promise.resolve();
    handler!({ payload: { runId: "run-old", event: { type: "info", message: "old" } } });
    handler!({ payload: { runId: "run-current", event: { type: "info", message: "current" } } });
    expect(progress).toHaveBeenCalledOnce();
    finishInvocation(summary);
    const result = await pending;
    handler!({ payload: { runId: "run-current", event: { type: "info", message: "after cleanup" } } });

    expect(order).toEqual(["listen", "invoke"]);
    expect(result).toBe(summary);
    expect(progress).toHaveBeenCalledOnce();
    expect(progress).toHaveBeenCalledWith({ type: "info", message: "current" });
    expect(cleanup).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("refine_cards_llm_tiered", expect.objectContaining({ runId: "run-current", cards, prompt: "prompt", batchMode: true }));
  });

  it("always cleans up when invocation fails", async () => {
    const cleanup = vi.fn();
    const invoke = vi.fn(async () => { throw new Error("invoke failed"); });
    await expect(runRefineAuto({
      cards, prompt: "", tiers: [], batchMode: false, isCancelled: () => false,
      onProgress: vi.fn(), subscribe: vi.fn(async () => cleanup) as unknown as RefineListen, invoke,
    })).rejects.toThrow("invoke failed");
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("does not invoke when listener registration fails", async () => {
    const invoke = vi.fn();
    await expect(runRefineAuto({
      cards, prompt: "", tiers: [], batchMode: false, isCancelled: () => false,
      onProgress: vi.fn(), subscribe: vi.fn(async () => { throw new Error("listen failed"); }) as unknown as RefineListen,
      invoke,
    })).rejects.toThrow("listen failed");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("cleans a late registration and skips invocation after cancellation", async () => {
    let resolve!: (cleanup: () => void) => void;
    const cleanup = vi.fn();
    let handler: ((event: any) => void) | undefined;
    const subscribe = vi.fn(async (_name: string, callback: (event: any) => void) => {
      handler = callback;
      return new Promise<() => void>(done => { resolve = done; });
    }) as unknown as RefineListen;
    const invoke = vi.fn();
    const progress = vi.fn();
    let cancelled = false;
    const pending = runRefineAuto({
      cards, prompt: "", tiers: [], batchMode: false, isCancelled: () => cancelled,
      onProgress: progress, subscribe, invoke, createRunId: () => "run-current",
    });
    cancelled = true;
    resolve(cleanup);

    expect(await pending).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
    expect(cleanup).toHaveBeenCalledOnce();
    handler!({ payload: { runId: "run-current", event: { type: "info", message: "late" } } });
    expect(progress).not.toHaveBeenCalled();
  });
});

import { expect, it, vi } from "vitest";
import { manageWindowSize, type ManagedWindow } from "./windowLifecycle";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((yes) => { resolve = yes; });
  return { promise, resolve };
}

function windowStub(overrides: Partial<ManagedWindow> = {}) {
  const base: ManagedWindow = {
    scaleFactor: vi.fn().mockResolvedValue(2),
    setMinSize: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    onResized: vi.fn().mockResolvedValue(vi.fn()),
  };
  return Object.assign(base, overrides);
}

it("unlistens when resize registration resolves after teardown", async () => {
  const registration = deferred<() => void>();
  const unlisten = vi.fn();
  const appWindow = windowStub({ onResized: vi.fn(() => registration.promise) });
  const onWidth = vi.fn();
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, onWidth);

  await vi.waitFor(() => expect(appWindow.onResized).toHaveBeenCalledOnce());
  stop();
  registration.resolve(unlisten);
  await vi.waitFor(() => expect(unlisten).toHaveBeenCalledOnce());

  const handler = vi.mocked(appWindow.onResized).mock.calls[0][0];
  await handler({ payload: { width: 2800, height: 1400 } });
  expect(onWidth).not.toHaveBeenCalled();
  expect(appWindow.setSize).not.toHaveBeenCalled();
});

it("does not continue setup when scale-factor lookup resolves after teardown", async () => {
  const scale = deferred<number>();
  const appWindow = windowStub({ scaleFactor: vi.fn(() => scale.promise) });
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, vi.fn());
  stop();
  scale.resolve(2);
  await Promise.resolve();

  expect(appWindow.setMinSize).not.toHaveBeenCalled();
  expect(appWindow.onResized).not.toHaveBeenCalled();
});

it("reports native setup failures to the caller", async () => {
  const error = new Error("resize listener unavailable");
  const appWindow = windowStub({ onResized: vi.fn().mockRejectedValue(error) });
  const onError = vi.fn();
  manageWindowSize(appWindow, { width: 460, height: 520 }, vi.fn(), onError);
  await vi.waitFor(() => expect(onError).toHaveBeenCalledWith(error));
});

it("enforces native minimum size and reports responsive widths while mounted", async () => {
  let handler!: (event: { payload: { width: number; height: number } }) => void | Promise<void>;
  const appWindow = windowStub({ onResized: vi.fn(async (listener) => { handler = listener; return vi.fn(); }) });
  const onWidth = vi.fn();
  manageWindowSize(appWindow, { width: 460, height: 520 }, onWidth);
  await vi.waitFor(() => expect(handler).toBeTypeOf("function"));

  await handler({ payload: { width: 1000, height: 700 } });
  expect(appWindow.setSize).toHaveBeenCalledOnce();
  expect(onWidth).toHaveBeenCalledWith(500);
});

it("reports native registration failure without an unhandled rejection", async () => {
  const error = new Error("window disconnected");
  const onError = vi.fn();
  const appWindow = windowStub({ onResized: vi.fn().mockRejectedValue(error) });
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, vi.fn(), onError);
  await vi.waitFor(() => expect(onError).toHaveBeenCalledExactlyOnceWith(error));
  stop();
});

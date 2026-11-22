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

it("updates minimum size limits when monitor scale factor changes", async () => {
  let resizeHandler!: (event: { payload: { width: number; height: number } }) => void | Promise<void>;
  let scaleHandler!: (event: { payload: { scaleFactor: number; size: { width: number; height: number } } }) => void | Promise<void>;
  const stopScale = vi.fn();
  const appWindow = windowStub({
    scaleFactor: vi.fn().mockResolvedValue(2),
    onResized: vi.fn(async (listener) => { resizeHandler = listener; return vi.fn(); }),
    onScaleChanged: vi.fn(async (listener) => { scaleHandler = listener; return stopScale; }),
  });
  const onWidth = vi.fn();
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, onWidth);
  await vi.waitFor(() => expect(scaleHandler).toBeTypeOf("function"));

  expect(appWindow.setMinSize).toHaveBeenCalledWith(expect.objectContaining({ width: 920, height: 1040 }));

  await scaleHandler({ payload: { scaleFactor: 1, size: { width: 800, height: 700 } } });
  expect(appWindow.setMinSize).toHaveBeenLastCalledWith(expect.objectContaining({ width: 460, height: 520 }));
  expect(appWindow.setSize).not.toHaveBeenCalled();
  expect(onWidth).toHaveBeenLastCalledWith(800);

  await resizeHandler({ payload: { width: 800, height: 700 } });
  expect(appWindow.setSize).not.toHaveBeenCalled();

  await scaleHandler({ payload: { scaleFactor: 2, size: { width: 800, height: 700 } } });
  expect(appWindow.setMinSize).toHaveBeenLastCalledWith(expect.objectContaining({ width: 920, height: 1040 }));
  expect(appWindow.setSize).toHaveBeenCalledWith(expect.objectContaining({ width: 920, height: 1040 }));

  stop();
  expect(stopScale).toHaveBeenCalledOnce();
});

it("unlistens scale-change registration when it resolves after teardown", async () => {
  const scaleReg = deferred<() => void>();
  const stopScale = vi.fn();
  const appWindow = windowStub({
    onScaleChanged: vi.fn(() => scaleReg.promise),
  });
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, vi.fn());
  await vi.waitFor(() => expect(appWindow.onScaleChanged).toHaveBeenCalledOnce());
  stop();
  scaleReg.resolve(stopScale);
  await vi.waitFor(() => expect(stopScale).toHaveBeenCalledOnce());
});

it("ignores stale scale-change events superseded while setMinSize is pending", async () => {
  let scaleHandler!: (event: { payload: { scaleFactor: number; size: { width: number; height: number } } }) => void | Promise<void>;
  const minSizeGates = [deferred<void>(), deferred<void>(), deferred<void>()];
  let gateIndex = 0;
  const appWindow = windowStub({
    scaleFactor: vi.fn().mockResolvedValue(2),
    setMinSize: vi.fn(() => minSizeGates[gateIndex++].promise),
    onScaleChanged: vi.fn(async (listener) => { scaleHandler = listener; return vi.fn(); }),
  });
  const onWidth = vi.fn();
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, onWidth);

  minSizeGates[0].resolve();
  await vi.waitFor(() => expect(scaleHandler).toBeTypeOf("function"));

  const firstChange = scaleHandler({ payload: { scaleFactor: 1, size: { width: 800, height: 700 } } });
  const secondChange = scaleHandler({ payload: { scaleFactor: 2, size: { width: 1600, height: 1400 } } });

  minSizeGates[1].resolve();
  await firstChange;
  expect(appWindow.setSize).not.toHaveBeenCalled();
  expect(onWidth).not.toHaveBeenCalled();

  minSizeGates[2].resolve();
  await secondChange;
  expect(appWindow.setSize).not.toHaveBeenCalled();
  expect(onWidth).toHaveBeenCalledExactlyOnceWith(800);

  stop();
});

it.each(["minimum", "resize"])("discards dimensions superseded by a resize during %s update", async (phase) => {
  const gate = deferred<void>();
  const appWindow = windowStub({ onScaleChanged: vi.fn().mockResolvedValue(vi.fn()) });
  const onWidth = vi.fn();
  const stop = manageWindowSize(appWindow, { width: 460, height: 520 }, onWidth);
  await vi.waitFor(() => expect(appWindow.onScaleChanged).toHaveBeenCalledOnce());
  const scaleHandler = vi.mocked(appWindow.onScaleChanged!).mock.calls[0][0];
  const resizeHandler = vi.mocked(appWindow.onResized).mock.calls[0][0];
  let pending: void | Promise<void>;
  if (phase === "minimum") {
    vi.mocked(appWindow.setMinSize).mockImplementationOnce(() => gate.promise);
    pending = scaleHandler({ payload: { scaleFactor: 2, size: { width: 1000, height: 700 } } });
  } else {
    vi.mocked(appWindow.setSize).mockImplementationOnce(() => gate.promise);
    pending = resizeHandler({ payload: { width: 1000, height: 700 } });
  }
  await resizeHandler({ payload: { width: 1800, height: 1400 } });
  gate.resolve();
  await pending;
  if (phase === "minimum") expect(appWindow.setSize).not.toHaveBeenCalled();
  expect(onWidth).toHaveBeenCalledExactlyOnceWith(900);
  stop();
});

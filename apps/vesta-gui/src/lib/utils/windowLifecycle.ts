import { PhysicalSize } from "@tauri-apps/api/dpi";

type WindowSize = { width: number; height: number };
type ResizeEvent = { payload: WindowSize };
type ScaleChangedEvent = { payload: { scaleFactor: number; size: WindowSize } };
export type ManagedWindow = {
  scaleFactor(): Promise<number>;
  setMinSize(size: PhysicalSize): Promise<void>;
  setSize(size: PhysicalSize): Promise<void>;
  onResized(handler: (event: ResizeEvent) => void | Promise<void>): Promise<() => void>;
  onScaleChanged?: (handler: (event: ScaleChangedEvent) => void | Promise<void>) => Promise<() => void>;
};

/** Enforce native minimum dimensions and release listeners even when setup resolves after teardown. */
export function manageWindowSize(
  appWindow: ManagedWindow,
  minimum: { width: number; height: number },
  onLogicalWidth: (width: number) => void,
  onError: (error: unknown) => void = (error) => console.error("Failed to manage native window size", error),
): () => void {
  let disposed = false;
  const unlisteners: Array<() => void> = [];

  const cleanup = () => {
    disposed = true;
    for (const stop of unlisteners.splice(0)) stop();
  };

  const registerListener = (stop: () => void) => {
    if (disposed) stop();
    else unlisteners.push(stop);
  };

  void (async () => {
    let currentScaleFactor = 1;
    let physicalMinWidth = minimum.width;
    let physicalMinHeight = minimum.height;
    let geometryGeneration = 0;

    const updateScale = (nextScale: number) => {
      if (!Number.isFinite(nextScale) || nextScale <= 0) return;
      currentScaleFactor = nextScale;
      physicalMinWidth = Math.round(minimum.width * currentScaleFactor);
      physicalMinHeight = Math.round(minimum.height * currentScaleFactor);
    };

    const enforceWindowSize = async (size: WindowSize, expectedGeneration = geometryGeneration) => {
      if (disposed || expectedGeneration !== geometryGeneration || size.width === 0 || size.height === 0) return;
      const minWidth = physicalMinWidth;
      const minHeight = physicalMinHeight;
      const scale = currentScaleFactor;
      if (size.width < minWidth || size.height < minHeight) {
        await appWindow
          .setSize(new PhysicalSize(Math.max(size.width, minWidth), Math.max(size.height, minHeight)))
          .catch(() => {});
      }
      if (!disposed && expectedGeneration === geometryGeneration) {
        const logicalWidth = size.width / scale;
        if (logicalWidth >= minimum.width) onLogicalWidth(logicalWidth);
      }
    };

    updateScale(await appWindow.scaleFactor());
    if (disposed) return;
    await appWindow.setMinSize(new PhysicalSize(physicalMinWidth, physicalMinHeight)).catch(() => {});
    if (disposed) return;

    const stopResize = await appWindow.onResized(async ({ payload: size }) => {
      if (disposed) return;
      await enforceWindowSize(size, ++geometryGeneration);
    });
    registerListener(stopResize);
    if (disposed) return;

    if (appWindow.onScaleChanged) {
      const stopScale = await appWindow.onScaleChanged(async ({ payload }) => {
        if (disposed) return;
        const generation = ++geometryGeneration;
        updateScale(payload.scaleFactor);
        await appWindow.setMinSize(new PhysicalSize(physicalMinWidth, physicalMinHeight)).catch(() => {});
        if (disposed || generation !== geometryGeneration) return;
        await enforceWindowSize(payload.size, generation);
      });
      registerListener(stopScale);
    }
  })().catch((error) => {
    cleanup();
    onError(error);
  });

  return cleanup;
}

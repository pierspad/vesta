import { PhysicalSize } from "@tauri-apps/api/dpi";

type WindowSize = { width: number; height: number };
type ResizeEvent = { payload: WindowSize };
export type ManagedWindow = {
  scaleFactor(): Promise<number>;
  setMinSize(size: PhysicalSize): Promise<void>;
  setSize(size: PhysicalSize): Promise<void>;
  onResized(handler: (event: ResizeEvent) => void | Promise<void>): Promise<() => void>;
};

/** Enforce native minimum dimensions and release a listener even when setup resolves after teardown. */
export function manageWindowSize(
  appWindow: ManagedWindow,
  minimum: { width: number; height: number },
  onLogicalWidth: (width: number) => void,
  onError: (error: unknown) => void = (error) => console.error("Failed to manage native window size", error),
): () => void {
  let disposed = false;
  let unlisten: (() => void) | undefined;

  void (async () => {
    const scaleFactor = await appWindow.scaleFactor();
    if (disposed) return;
    const physicalMinWidth = Math.round(minimum.width * scaleFactor);
    const physicalMinHeight = Math.round(minimum.height * scaleFactor);
    await appWindow.setMinSize(new PhysicalSize(physicalMinWidth, physicalMinHeight)).catch(() => {});
    if (disposed) return;

    const stopListening = await appWindow.onResized(async ({ payload: size }) => {
      if (disposed || size.width === 0 || size.height === 0) return;
      if (size.width < physicalMinWidth || size.height < physicalMinHeight) {
        await appWindow.setSize(new PhysicalSize(Math.max(size.width, physicalMinWidth), Math.max(size.height, physicalMinHeight))).catch(() => {});
      }
      if (!disposed) {
        const logicalWidth = size.width / scaleFactor;
        if (logicalWidth >= minimum.width) onLogicalWidth(logicalWidth);
      }
    });
    if (disposed) stopListening();
    else unlisten = stopListening;
  })().catch(onError);

  return () => {
    disposed = true;
    unlisten?.();
    unlisten = undefined;
  };
}

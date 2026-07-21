import { getCurrentWebview } from "@tauri-apps/api/webview";

export interface DragDropHandlers {
  /** Called with the dropped file paths (non-empty). */
  onDrop: (paths: string[]) => void;
  /** Reactive setter for "a file is currently hovering over the window". */
  setDraggingOver: (value: boolean) => void;
  /** Re-checked on every event; skip handling entirely when it returns false
   *  (e.g. this tab isn't the active one, so its cross-window listener
   *  shouldn't react to drops meant for another tab). Not needed when the
   *  caller already scopes registration to the active state via `$effect`. */
  isActive?: () => boolean;
  /** Defaults to a `console.warn`. */
  onError?: (e: unknown) => void;
}

/**
 * Registers Tauri's window-level `onDragDropEvent` listener and wires it to
 * `setDraggingOver`/`onDrop`.
 *
 * Every tab needs this because Tauri reports drag-and-drop at the *window*
 * level, not per-DOM-element — each tab has to register its own listener and
 * filter by `active` itself. Before this helper, that ~20-line dance (register,
 * track the in-flight registration promise, guard against unmounting before
 * it resolves, unlisten on cleanup) was duplicated near-verbatim in six tabs.
 *
 * Call from `onMount` or a Svelte 5 `$effect`, and call the returned cleanup
 * function from the teardown callback:
 *
 * ```svelte
 * onMount(() => {
 *   const cleanupDragDrop = setupWebviewDragDrop({
 *     isActive: () => active,
 *     setDraggingOver: (v) => (isDraggingOver = v),
 *     onDrop: handleDroppedFiles,
 *   });
 *   return () => cleanupDragDrop();
 * });
 * ```
 */
export function setupWebviewDragDrop(handlers: DragDropHandlers): () => void {
  let stillMounted = true;
  let unlistenFn: (() => void) | null = null;

  getCurrentWebview()
    .onDragDropEvent((event) => {
      if (handlers.isActive && !handlers.isActive()) return;
      if (event.payload.type === "over") {
        handlers.setDraggingOver(true);
      } else if (event.payload.type === "drop") {
        handlers.setDraggingOver(false);
        if (event.payload.paths && event.payload.paths.length > 0) {
          handlers.onDrop(event.payload.paths);
        }
      } else if (event.payload.type === "leave") {
        handlers.setDraggingOver(false);
      }
    })
    .then((fn) => {
      // The component (or the $effect) may have already torn down by the
      // time Tauri resolves this registration — unlisten immediately
      // instead of leaking a listener that outlives its owner.
      if (!stillMounted) fn();
      else unlistenFn = fn;
    })
    .catch((e) => {
      if (handlers.onError) handlers.onError(e);
      else console.warn("Failed to set up drag-drop listener:", e);
    });

  return () => {
    stillMounted = false;
    if (unlistenFn) unlistenFn();
  };
}

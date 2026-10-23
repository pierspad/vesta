/** Measure translated menus rather than guessing their height near window edges. */
export function contextMenu(node: HTMLElement, options: { x: number; y: number; close: () => void }) {
  const previousFocus = document.activeElement;
  node.setAttribute("role", "menu");
  node.tabIndex = -1;
  Object.assign(node.style, { position: "fixed", maxWidth: "calc(100vw - 16px)", maxHeight: "calc(100vh - 16px)", overflowY: "auto" });
  const buttons = () => [...node.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  function position() {
    const box = node.getBoundingClientRect();
    node.style.left = `${Math.max(8, Math.min(options.x, window.innerWidth - box.width - 8))}px`;
    node.style.top = `${Math.max(8, Math.min(options.y, window.innerHeight - box.height - 8))}px`;
  }
  function keydown(event: KeyboardEvent) {
    if (event.key === "Escape" || event.key === "Tab") {
      if (event.key === "Escape") event.preventDefault();
      event.stopPropagation(); options.close(); return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    const items = buttons();
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
    items[next]?.focus();
  }
  position();
  node.querySelectorAll("button").forEach(button => button.setAttribute("role", "menuitem"));
  (buttons()[0] ?? node).focus({ preventScroll: true });
  node.addEventListener("keydown", keydown);
  window.addEventListener("resize", position);
  const observer = typeof ResizeObserver !== "undefined" ? new ResizeObserver(position) : null;
  observer?.observe(node);
  return {
    update(value: typeof options) { options = value; position(); },
    destroy() {
      observer?.disconnect(); window.removeEventListener("resize", position); node.removeEventListener("keydown", keydown);
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus({ preventScroll: true });
    },
  };
}

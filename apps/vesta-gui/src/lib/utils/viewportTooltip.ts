/** Portal existing tooltip content and keep it inside the visible webview. */
export function viewportTooltip(node: HTMLElement, options: { visible?: boolean } = {}) {
  const anchor = node.parentElement!;
  const trigger = anchor.querySelector<HTMLElement>("button, input, [tabindex]") || anchor;
  let hovered = false;
  let focused = false;
  let forced = !!options.visible;
  const oldStyle = node.getAttribute("style");
  const oldRole = node.getAttribute("role");
  node.setAttribute("role", "tooltip");
  document.body.append(node);
  function position() {
    const bounds = trigger.getBoundingClientRect();
    const margin = 8;
    Object.assign(node.style, { position: "fixed", inset: "auto", margin: "0", transform: "none", zIndex: "10000",
      maxWidth: `calc(100vw - ${margin * 2}px)`, maxHeight: `calc(100vh - ${margin * 2}px)`,
      whiteSpace: "normal", overflowWrap: "anywhere", transition: "none", pointerEvents: "none" });
    const box = node.getBoundingClientRect();
    const left = Math.max(margin, Math.min(window.innerWidth - box.width - margin, bounds.left + (bounds.width - box.width) / 2));
    const above = bounds.top - box.height - margin;
    const top = Math.max(margin, Math.min(window.innerHeight - box.height - margin, above >= margin ? above : bounds.bottom + margin));
    node.style.left = `${left}px`; node.style.top = `${top}px`;
  }
  function render() {
    const visible = hovered || focused || forced;
    node.style.display = visible ? "block" : "none";
    node.style.opacity = visible ? "1" : "0";
    if (visible) position();
  }
  const enter = () => { hovered = true; render(); };
  const leave = () => { hovered = false; render(); };
  const focus = () => { focused = true; render(); };
  const blur = (event: FocusEvent) => { if (!anchor.contains(event.relatedTarget as Node)) { focused = false; render(); } };
  const escape = (event: KeyboardEvent) => { if (event.key === "Escape") { hovered = focused = forced = false; render(); } };
  anchor.addEventListener("mouseenter", enter); anchor.addEventListener("mouseleave", leave);
  anchor.addEventListener("focusin", focus); anchor.addEventListener("focusout", blur);
  window.addEventListener("keydown", escape); window.addEventListener("resize", render); window.addEventListener("scroll", render, true);
  render();
  return { update(value: { visible?: boolean }) { forced = !!value?.visible; render(); }, destroy() {
    anchor.removeEventListener("mouseenter", enter); anchor.removeEventListener("mouseleave", leave);
    anchor.removeEventListener("focusin", focus); anchor.removeEventListener("focusout", blur);
    window.removeEventListener("keydown", escape); window.removeEventListener("resize", render); window.removeEventListener("scroll", render, true);
    if (oldStyle === null) node.removeAttribute("style"); else node.setAttribute("style", oldStyle);
    if (oldRole === null) node.removeAttribute("role"); else node.setAttribute("role", oldRole);
    node.remove();
  } };
}

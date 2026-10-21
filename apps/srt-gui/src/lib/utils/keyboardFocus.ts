/** Shared focus guard for application shortcuts. */
export function isInputActive(): boolean {
  const active = document.activeElement;
  if (!active) return false;
  const tag = active.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || active.getAttribute("contenteditable") === "true";
}

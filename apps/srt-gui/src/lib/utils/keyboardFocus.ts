/** Shared focus guard for application shortcuts. */
export function isInputActive(): boolean {
  return !!document.activeElement?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
}

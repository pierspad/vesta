export interface FocusTrapOptions {
  enabled?: boolean;
  onEscape?: () => void;
  restoreFocus?: boolean;
}

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not(:disabled)",
  'input:not(:disabled):not([type="hidden"])',
  "select:not(:disabled)",
  "textarea:not(:disabled)",
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(", ");

const activeTrapStack: symbol[] = [];

function isElementFocusable(el: HTMLElement): boolean {
  if (el.hidden || el.hasAttribute("disabled") || el.getAttribute("tabindex") === "-1") {
    return false;
  }
  if (el.closest('[aria-hidden="true"], [inert]')) {
    return false;
  }
  const style = window.getComputedStyle(el);
  return style.display !== "none" && style.visibility !== "hidden";
}

export function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    isElementFocusable,
  );
}

/**
 * Traps keyboard Tab navigation inside a modal dialog container, optionally handles Escape,
 * and restores focus to the previously focused element when deactivated or destroyed.
 */
export function focusTrap(node: HTMLElement, options: FocusTrapOptions = {}) {
  const token = Symbol("focusTrap");
  let current = { enabled: true, restoreFocus: true, ...options };
  let previousFocus: HTMLElement | null = null;
  let active = false;

  const focusFirst = () => {
    const focusable = getFocusableElements(node);
    if (focusable.length > 0) {
      focusable[0].focus();
      return;
    }
    if (!node.hasAttribute("tabindex")) {
      node.setAttribute("tabindex", "-1");
    }
    node.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (!current.enabled || event.defaultPrevented) return;
    if (activeTrapStack[activeTrapStack.length - 1] !== token) return;

    if (event.key === "Escape" && current.onEscape && !event.isComposing) {
      event.preventDefault();
      event.stopPropagation();
      current.onEscape();
      return;
    }

    if (event.key !== "Tab") return;

    const focusable = getFocusableElements(node);
    if (focusable.length === 0) {
      event.preventDefault();
      node.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    const activeEl = document.activeElement as HTMLElement | null;

    if (!activeEl || !node.contains(activeEl)) {
      event.preventDefault();
      (event.shiftKey ? last : first).focus();
      return;
    }

    if (event.shiftKey && (activeEl === first || activeEl === node)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && activeEl === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const activate = () => {
    if (active) return;
    active = true;
    const prev = document.activeElement;
    previousFocus = prev instanceof HTMLElement && !node.contains(prev) ? prev : null;
    activeTrapStack.push(token);
    document.addEventListener("keydown", onKeyDown, false);
    if (!node.contains(document.activeElement)) {
      focusFirst();
    }
  };

  const deactivate = () => {
    if (!active) return;
    active = false;
    document.removeEventListener("keydown", onKeyDown, false);
    const idx = activeTrapStack.lastIndexOf(token);
    if (idx !== -1) {
      activeTrapStack.splice(idx, 1);
    }
    const target = previousFocus;
    previousFocus = null;
    if (current.restoreFocus !== false && target && target.isConnected && !node.contains(target)) {
      target.focus();
    }
  };

  if (current.enabled !== false) {
    activate();
  }

  return {
    update(next: FocusTrapOptions = {}) {
      current = { enabled: true, restoreFocus: true, ...next };
      if (current.enabled !== false) {
        activate();
      } else {
        deactivate();
      }
    },
    destroy() {
      deactivate();
    },
  };
}

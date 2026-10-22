import { locale } from "$lib/i18n";

/** Keep native number validation/bindings while replacing platform spin buttons. */
export function numberControl(input: HTMLInputElement) {
  const wrapper = document.createElement("span");
  wrapper.className = "number-control";
  input.before(wrapper); wrapper.append(input);
  const controls = document.createElement("span"); controls.className = "number-control-buttons";
  wrapper.append(controls);
  function step(direction: number, multiplier = 1) {
    if (input.disabled || input.readOnly) return;
    if (!Number.isFinite(input.valueAsNumber)) input.value = input.min || "0";
    direction > 0 ? input.stepUp(multiplier) : input.stepDown(multiplier);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }
  const buttons = [1, -1].map(direction => {
    const button = document.createElement("button");
    button.type = "button"; button.tabIndex = -1;
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24"); svg.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(svg.namespaceURI, "path");
    path.setAttribute("d", direction > 0 ? "m7 14 5-5 5 5" : "m7 10 5 5 5-5");
    svg.append(path); button.append(svg);
    button.onpointerdown = event => event.preventDefault();
    button.onclick = event => { input.focus(); step(direction, event.ctrlKey || event.metaKey ? 100 : event.shiftKey ? 10 : 1); };
    controls.append(button); return button;
  });
  const unsubscribe = locale.subscribe(t => {
    buttons.forEach((button, index) => {
      button.setAttribute("aria-label", t(index === 0 ? "common.increaseValue" : "common.decreaseValue"));
      button.title = `${button.getAttribute("aria-label")} · Shift ×10 · Ctrl ×100`;
    });
  });
  const sync = () => buttons.forEach(button => button.disabled = input.disabled || input.readOnly);
  const observer = new MutationObserver(sync);
  observer.observe(input, { attributes: true, attributeFilter: ["disabled", "readonly"] }); sync();
  const onKey = (event: KeyboardEvent) => {
    if (event.altKey || event.isComposing || !["ArrowUp", "ArrowRight", "ArrowDown", "ArrowLeft"].includes(event.key)) return;
    if (input.disabled || input.readOnly) return;
    event.preventDefault(); event.stopPropagation();
    step(["ArrowUp", "ArrowRight"].includes(event.key) ? 1 : -1, event.ctrlKey || event.metaKey ? 100 : event.shiftKey ? 10 : 1);
  };
  input.addEventListener("keydown", onKey);
  return { destroy() {
    unsubscribe(); observer.disconnect(); input.removeEventListener("keydown", onKey);
    wrapper.replaceWith(input);
  } };
}

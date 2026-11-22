/** Suspend decorative animations during native scrolling; no per-frame layout reads. */
export function settledScroll(node: HTMLElement) {
  let timer: ReturnType<typeof setTimeout>;
  const onScroll = () => {
    node.classList.add("is-scrolling");
    clearTimeout(timer);
    timer = setTimeout(() => node.classList.remove("is-scrolling"), 140);
  };
  node.addEventListener("scroll", onScroll, { passive: true });
  return { destroy() { clearTimeout(timer); node.removeEventListener("scroll", onScroll); } };
}

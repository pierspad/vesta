<script lang="ts">
  import { onMount } from "svelte";
  let text = $state("");
  let x = $state(0);
  let y = $state(0);
  const tooltipId = "app-tooltip";
  onMount(() => {
    let owner: HTMLElement | null = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    function hide() {
      clearTimeout(timer);
      if (owner) {
        if (!owner.hasAttribute("title")) owner.title = owner.dataset.vestaTitle || "";
        delete owner.dataset.vestaTitle;
        const describedBy = owner.getAttribute("aria-describedby")?.split(/\s+/).filter(id => id && id !== tooltipId) || [];
        if (describedBy.length) owner.setAttribute("aria-describedby", describedBy.join(" "));
        else owner.removeAttribute("aria-describedby");
      }
      owner = null;
      text = "";
    }
    function show(event: Event) {
      const target = event.target instanceof Element ? event.target.closest<HTMLElement>("[title], [data-vesta-title]") : null;
      if (target === owner) {
        // A keyboard user may focus the same element while its hover delay is
        // pending. Promote that pending tooltip to an immediate focus tooltip.
        if (event.type === "focusin" && owner) {
          clearTimeout(timer);
          const label = owner.dataset.vestaTitle || "";
          const rect = owner.getBoundingClientRect();
          x = Math.max(12, Math.min(rect.left, window.innerWidth - Math.min(360, window.innerWidth - 24) - 12));
          y = rect.bottom + 8;
          if (y + 90 > window.innerHeight) y = Math.max(12, rect.top - 80);
          text = label;
        }
        return;
      }
      hide();
      if (!target?.title) return;
      owner = target;
      const label = target.title;
      target.dataset.vestaTitle = label;
      target.removeAttribute("title");
      const describedBy = target.getAttribute("aria-describedby")?.split(/\s+/).filter(Boolean) || [];
      if (!describedBy.includes(tooltipId)) describedBy.push(tooltipId);
      target.setAttribute("aria-describedby", describedBy.join(" "));
      const display = () => {
        const rect = target.getBoundingClientRect();
        x = Math.max(12, Math.min(rect.left, window.innerWidth - Math.min(360, window.innerWidth - 24) - 12));
        y = rect.bottom + 8;
        if (y + 90 > window.innerHeight) y = Math.max(12, rect.top - 80);
        text = label;
      };
      // Keyboard focus should expose the description immediately to assistive technology.
      if (event.type === "focusin") display();
      else timer = setTimeout(display, 450);
    }
    function leave(event: MouseEvent) {
      if (owner && !(event.relatedTarget instanceof Node && owner.contains(event.relatedTarget))) hide();
    }
    document.addEventListener("mouseover", show);
    document.addEventListener("focusin", show);
    document.addEventListener("mouseout", leave);
    document.addEventListener("focusout", hide);
    document.addEventListener("pointerdown", hide);
    document.addEventListener("keydown", hide);
    document.addEventListener("scroll", hide, true);
    return () => {
      hide();
      document.removeEventListener("mouseover", show);
      document.removeEventListener("focusin", show);
      document.removeEventListener("mouseout", leave);
      document.removeEventListener("focusout", hide);
      document.removeEventListener("pointerdown", hide);
      document.removeEventListener("keydown", hide);
      document.removeEventListener("scroll", hide, true);
    };
  });
</script>
<div id={tooltipId} role="tooltip" class="app-tooltip" class:visible={!!text} style:left={`${x}px`} style:top={`${y}px`}>{text}</div>
<style>
  .app-tooltip {
    position: fixed;
    z-index: 10000;
    max-width: min(360px, calc(100vw - 24px));
    pointer-events: none;
    visibility: hidden;
    border: 1px solid rgb(255 255 255 / 0.12);
    border-radius: 8px;
    background: rgb(17 24 39 / 0.98);
    padding: 7px 10px;
    color: #e5e7eb;
    font-size: 12px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.3);
  }
  .app-tooltip.visible { visibility: visible; }
</style>

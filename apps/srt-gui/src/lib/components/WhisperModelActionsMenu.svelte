<script lang="ts">
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import type { WhisperModel } from "$lib/services/transcribe";
  let { model, busy, x, y, oninstall, onuninstall, onclose }: {
    model: WhisperModel; busy: boolean; x: number; y: number;
    oninstall: () => void; onuninstall: () => void; onclose: () => void;
  } = $props();
  let t = $derived($locale);
  let menu = $state<HTMLDivElement | null>(null);
  let left = $state(0);
  let top = $state(0);
  onMount(() => {
    const previousFocus = document.activeElement;
    const rect = menu?.getBoundingClientRect();
    left = Math.max(8, Math.min(x, window.innerWidth - (rect?.width ?? 210) - 8));
    top = Math.max(8, Math.min(y, window.innerHeight - (rect?.height ?? 140) - 8));
    menu?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); onclose(); }
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const buttons = [...(menu?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        buttons[(index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length]?.focus();
      }
    };
    document.addEventListener("keydown", handleKey);
    return () => { document.removeEventListener("keydown", handleKey); if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus(); };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="fixed inset-0 z-[90]" onmousedown={onclose} oncontextmenu={(event) => { event.preventDefault(); onclose(); }}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div bind:this={menu} role="menu" tabindex="-1" aria-label={`Whisper ${model.name}`} class="absolute min-w-[210px] overflow-hidden rounded-xl border border-white/10 bg-gray-900/98 py-1 shadow-2xl" style="left: {left}px; top: {top}px" onmousedown={(event) => event.stopPropagation()} oncontextmenu={(event) => { event.preventDefault(); event.stopPropagation(); }}>
    <div class="mb-1 border-b border-white/5 bg-white/5 px-4 py-2 text-xs font-semibold text-gray-400">{t("transcribe.whisperModel")} · {model.name}</div>
    <button type="button" role="menuitem" disabled={busy || model.downloaded} onclick={oninstall} class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm text-cyan-300 hover:bg-cyan-500/10 disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent">
      <svg aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M12 3v12m-4-4 4 4 4-4M4 16v4h16v-4" /></svg>{t("transcribe.installModel")}
    </button>
    <button type="button" role="menuitem" disabled={busy || !model.downloaded} onclick={onuninstall} class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm text-red-300 hover:bg-red-500/10 disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent">
      <svg aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 7h16M10 11v6m4-6v6M6 7l1 14h10l1-14M9 7V3h6v4" /></svg>{t("settings.uninstall")}
    </button>
  </div>
</div>

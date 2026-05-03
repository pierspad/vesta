<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    getStoredSettingsActionState,
    markCurrentSettingsActionRead,
  } from "./settingsNotifications";

  type MenuState = {
    x: number;
    y: number;
    editable: HTMLInputElement | HTMLTextAreaElement | HTMLElement | null;
    settingsTarget: boolean;
    hasSelection: boolean;
    settingsHash: string;
  };

  let menu = $state<MenuState | null>(null);

  function getEditableTarget(target: EventTarget | null) {
    if (!(target instanceof HTMLElement)) return null;
    const editable = target.closest("input, textarea, [contenteditable='true']");
    if (editable instanceof HTMLInputElement || editable instanceof HTMLTextAreaElement) return editable;
    if (editable instanceof HTMLElement && editable.isContentEditable) return editable;
    return null;
  }

  function selectedText(target: MenuState["editable"]): string {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? 0;
      const end = target.selectionEnd ?? 0;
      return target.value.slice(start, end);
    }
    return window.getSelection()?.toString() || "";
  }

  function replaceSelection(target: MenuState["editable"], text: string) {
    if (!target) return;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      const start = target.selectionStart ?? target.value.length;
      const end = target.selectionEnd ?? target.value.length;
      target.value = `${target.value.slice(0, start)}${text}${target.value.slice(end)}`;
      target.selectionStart = target.selectionEnd = start + text.length;
      target.dispatchEvent(new Event("input", { bubbles: true }));
      target.focus();
      return;
    }

    target.focus();
    document.execCommand("insertText", false, text);
  }

  function closeMenu() {
    menu = null;
  }

  function openMenu(event: MouseEvent) {
    if (event.defaultPrevented) return;
    event.preventDefault();

    const settingsState = getStoredSettingsActionState();
    const target = event.target instanceof HTMLElement ? event.target : null;
    const editable = getEditableTarget(event.target);
    const settingsTarget = Boolean(target?.closest("[data-context-menu='settings-notifications']"));

    menu = {
      x: Math.min(event.clientX, window.innerWidth - 240),
      y: Math.min(event.clientY, window.innerHeight - 220),
      editable,
      settingsTarget,
      hasSelection: selectedText(editable).length > 0 || Boolean(window.getSelection()?.toString()),
      settingsHash: settingsState.hash,
    };
  }

  async function copy() {
    if (!menu) return;
    const text = selectedText(menu.editable) || window.getSelection()?.toString() || "";
    if (text) await navigator.clipboard.writeText(text);
    closeMenu();
  }

  async function cut() {
    if (!menu?.editable) return;
    const text = selectedText(menu.editable);
    if (text) await navigator.clipboard.writeText(text);
    replaceSelection(menu.editable, "");
    closeMenu();
  }

  async function paste() {
    if (!menu?.editable) return;
    replaceSelection(menu.editable, await navigator.clipboard.readText());
    closeMenu();
  }

  function selectAll() {
    if (!menu?.editable) return;
    if (menu.editable instanceof HTMLInputElement || menu.editable instanceof HTMLTextAreaElement) {
      menu.editable.focus();
      menu.editable.select();
    } else {
      const range = document.createRange();
      range.selectNodeContents(menu.editable);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
    closeMenu();
  }

  function markSettingsRead() {
    markCurrentSettingsActionRead();
    closeMenu();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") closeMenu();
  }

  onMount(() => {
    window.addEventListener("contextmenu", openMenu);
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("contextmenu", openMenu);
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

{#if menu}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-[90]"
    onmousedown={closeMenu}
    oncontextmenu={(event) => {
      event.preventDefault();
      closeMenu();
    }}
  >
    <div
      class="vesta-context-menu"
      style="left: {menu.x}px; top: {menu.y}px;"
      onmousedown={(event) => event.stopPropagation()}
    >
      {#if menu.settingsTarget}
        <button
          type="button"
          class="vesta-context-menu-item"
          disabled={!menu.settingsHash}
          onclick={markSettingsRead}
        >
          <span>{menu.settingsHash ? "Rimuovi notifiche impostazioni" : "Nessuna notifica impostazioni"}</span>
        </button>
        <div class="vesta-context-menu-separator"></div>
      {/if}

      <button type="button" class="vesta-context-menu-item" disabled={!menu.hasSelection} onclick={copy}>
        <span>Copia</span>
        <kbd>Ctrl C</kbd>
      </button>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable || !menu.hasSelection} onclick={cut}>
        <span>Taglia</span>
        <kbd>Ctrl X</kbd>
      </button>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable} onclick={paste}>
        <span>Incolla</span>
        <kbd>Ctrl V</kbd>
      </button>
      <div class="vesta-context-menu-separator"></div>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable} onclick={selectAll}>
        <span>Seleziona tutto</span>
        <kbd>Ctrl A</kbd>
      </button>
    </div>
  </div>
{/if}

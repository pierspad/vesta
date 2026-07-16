<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { locale } from "./i18n";
  import {
    getStoredSettingsActionState,
    hideSettingsNotifications,
    showSettingsNotifications,
    markSettingsNotificationRead,
    unmarkSettingsNotificationRead,
  } from "./settingsNotifications";

  let t = $derived($locale);

  type MenuState = {
    x: number;
    y: number;
    editable: HTMLInputElement | HTMLTextAreaElement | HTMLElement | null;
    settingsTarget: boolean;
    hasSelection: boolean;
    settingsHash: string;
    settingsHidden: boolean;
    settingsRead: boolean;
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
      y: Math.min(event.clientY, window.innerHeight - 280),
      editable,
      settingsTarget,
      hasSelection: selectedText(editable).length > 0 || Boolean(window.getSelection()?.toString()),
      settingsHash: settingsState.hash,
      settingsHidden: settingsState.hidden,
      settingsRead: settingsState.read,
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

  function hideNotifications() {
    hideSettingsNotifications();
    closeMenu();
  }

  function showNotificationsAction() {
    showSettingsNotifications();
    closeMenu();
  }

  function toggleReadNotification() {
    if (!menu) return;
    if (menu.settingsRead) {
      unmarkSettingsNotificationRead();
    } else {
      markSettingsNotificationRead();
    }
    closeMenu();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!menu) return;
    const key = event.key.toLowerCase();

    if (event.key === "Escape") {
      closeMenu();
      return;
    }

    if (menu.settingsTarget) {
      if (key === "h" && menu.settingsHash && !menu.settingsHidden) {
        event.preventDefault();
        event.stopPropagation();
        hideNotifications();
      } else if (key === "s" && menu.settingsHash && menu.settingsHidden) {
        event.preventDefault();
        event.stopPropagation();
        showNotificationsAction();
      } else if (key === "r" && menu.settingsHash) {
        event.preventDefault();
        event.stopPropagation();
        toggleReadNotification();
      }
    }

    // Capture standard single-key shortcuts when menu is active (letters without modifiers)
    const target = event.target as HTMLElement;
    const isEditing = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;
    if (!isEditing) {
      if (key === "c" && !event.ctrlKey && !event.metaKey && menu.hasSelection) {
        event.preventDefault();
        event.stopPropagation();
        void copy();
      } else if (key === "x" && !event.ctrlKey && !event.metaKey && menu.editable && menu.hasSelection) {
        event.preventDefault();
        event.stopPropagation();
        void cut();
      } else if (key === "v" && !event.ctrlKey && !event.metaKey && menu.editable) {
        event.preventDefault();
        event.stopPropagation();
        void paste();
      } else if (key === "a" && !event.ctrlKey && !event.metaKey && menu.editable) {
        event.preventDefault();
        event.stopPropagation();
        selectAll();
      }
    }
  }

  onMount(() => {
    window.addEventListener("contextmenu", openMenu);
    window.addEventListener("keydown", handleKeydown, true);
  });

  onDestroy(() => {
    window.removeEventListener("contextmenu", openMenu);
    window.removeEventListener("keydown", handleKeydown, true);
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
        {#if menu.settingsHash && !menu.settingsHidden}
          <!-- Notifications are visible → offer "Hide" -->
          <button
            type="button"
            class="vesta-context-menu-item"
            onclick={hideNotifications}
          >
            <span class="flex items-center gap-2">
              <svg class="w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-5 0-9.27-3.11-11-7.5a11.72 11.72 0 013.168-4.457M6.343 6.343A9.97 9.97 0 0112 5c5 0 9.27 3.11 11 7.5a11.72 11.72 0 01-4.168 4.457M6.343 6.343L3 3m3.343 3.343l2.829 2.829m5.657 5.657l2.828 2.828M3 3l18 18m-9-9a3 3 0 01-3-3" />
              </svg>
              {t("contextMenu.hideNotifications")}
            </span>
            <kbd>H</kbd>
          </button>
        {:else if menu.settingsHash && menu.settingsHidden}
          <!-- Notifications are hidden → offer "Show" -->
          <button
            type="button"
            class="vesta-context-menu-item"
            onclick={showNotificationsAction}
          >
            <span class="flex items-center gap-2">
              <svg class="w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
              </svg>
              {t("contextMenu.showNotifications")}
            </span>
            <kbd>S</kbd>
          </button>
        {:else}
          <button
            type="button"
            class="vesta-context-menu-item"
            disabled
          >
            <span class="flex items-center gap-2 opacity-50">
              <svg class="w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6 6 0 10-12 0v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0a3 3 0 11-6 0m6 0H9" />
              </svg>
              {t("contextMenu.noNotifications")}
            </span>
          </button>
        {/if}

        {#if menu.settingsHash}
          <button
            type="button"
            class="vesta-context-menu-item"
            onclick={toggleReadNotification}
          >
            <span class="flex items-center gap-2">
              {#if menu.settingsRead}
                <svg class="w-3.5 h-3.5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-emerald-300">{t("contextMenu.acknowledged")}</span>
              {:else}
                <svg class="w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
                {t("contextMenu.markAsRead")}
              {/if}
            </span>
            <kbd>R</kbd>
          </button>
        {/if}
        <div class="vesta-context-menu-separator"></div>
      {/if}

      <button type="button" class="vesta-context-menu-item" disabled={!menu.hasSelection} onclick={copy}>
        <span>{t("common.copy")}</span>
        <kbd>{t("shortcuts.hint.copy")}</kbd>
      </button>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable || !menu.hasSelection} onclick={cut}>
        <span>{t("common.cut")}</span>
        <kbd>{t("shortcuts.hint.cut")}</kbd>
      </button>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable} onclick={paste}>
        <span>{t("common.paste")}</span>
        <kbd>{t("shortcuts.hint.paste")}</kbd>
      </button>
      <div class="vesta-context-menu-separator"></div>
      <button type="button" class="vesta-context-menu-item" disabled={!menu.editable} onclick={selectAll}>
        <span>{t("common.selectAll")}</span>
        <kbd>{t("shortcuts.hint.selectAll")}</kbd>
      </button>
    </div>
  </div>
{/if}

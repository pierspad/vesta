<script lang="ts">
  import { onMount, untrack, type Snippet } from "svelte";
  import { get } from "svelte/store";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    pluginRender,
    pluginDispatchEvent,
    pluginSaveSettings,
    type WidgetNode,
    type ButtonWidget,
    type PluginSettingField,
    type PluginPatch,
    type PathInputWidget,
  } from "$lib/services/pluginClient";
  import { currentLanguage, locale } from "$lib/i18n";
  import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { invokeCommand } from "$lib/services/tauriClient";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import SearchableSelect from "$lib/components/SearchableSelect.svelte";
  import { languages, extendedLanguages, languageFlagUrl, localizedLanguageName } from "$lib/config/languages";

  interface Props {
    pluginId: string;
    screen?: string;
    active?: boolean;
    initialRoot?: WidgetNode | null;
    header?: Snippet<[Snippet, () => void, boolean]>;
    onExit?: () => void;
  }

  let {
    pluginId,
    screen = "main",
    active = true,
    initialRoot = null,
    header,
    onExit,
  }: Props = $props();

  let t = $derived($locale);
  let activeProgress = $derived(pluginsStore.getProgress(pluginId));

  let currentScreen = $state(untrack(() => screen));
  let previousScreens = $state<string[]>([]);

  let uiRoot = $state<WidgetNode | null>(null);
  let values = $state<Record<string, any>>({});
  let renderer: HTMLDivElement;
  let loading = $state(false);
  let actionLoading = $state<string | null>(null);
  let error = $state<string | null>(null);

  let lastRoot: WidgetNode | null = null;
  $effect(() => {
    if (initialRoot && initialRoot !== lastRoot) {
      lastRoot = initialRoot;
      uiRoot = initialRoot;
      collectDefaults(uiRoot);
      triggerPluginSnackbar({ root: initialRoot });
    }
  });

  function triggerPluginSnackbar(res: any): void {
    if (!res || typeof res !== "object") return;
    const item =
      res.snackbar ||
      res.notification ||
      res.root?.snackbar ||
      res.root?.notification ||
      (res.patch as any)?.snackbar ||
      (res.patch as any)?.notification;
    if (!item) return;

    const message = typeof item === "string" ? item : item.message || item.text || item.msg;
    if (!message || typeof message !== "string") return;

    const rawType = (typeof item === "object" ? (item.type || item.variant || item.level) : "info")?.toLowerCase?.() || "info";
    if (rawType === "success") {
      snackbar.success(message);
    } else if (rawType === "error" || rawType === "danger") {
      snackbar.error(message);
    } else if (rawType === "warning" || rawType === "warn") {
      snackbar.warning(message);
    } else {
      snackbar.info(message);
    }
  }

  // Initialize defaults from widget tree into `values`
  function collectDefaults(node: WidgetNode | null | undefined): void {
    if (!node) return;
    if ("id" in node && node.id) {
      const id = node.id;
      if (values[id] === undefined) {
        if ("defaultValue" in node && node.defaultValue !== undefined) {
          values[id] = node.defaultValue;
        } else if (node.type === "checkbox") {
          values[id] = false;
        } else if (node.type === "numberInput") {
          values[id] = 0;
        } else if (node.type === "textInput" || node.type === "textArea" || node.type === "passwordInput" || node.type === "pathInput") {
          values[id] = "";
        } else if (node.type === "languageSelect") {
          values[id] = "en";
        } else if (node.type === "segmented") {
          values[id] = (node as any).options?.[0]?.value ?? "";
        }
      }
    }
    if (node.type === "settings") {
      for (const field of node.fields) {
        if (values[field.id] === undefined && field.defaultValue !== undefined) {
          values[field.id] = field.defaultValue;
        }
      }
    }
    if ("children" in node && Array.isArray(node.children)) {
      for (const child of node.children) {
        collectDefaults(child);
      }
    }
  }

  function applyPatchToTree(root: WidgetNode, patch: PluginPatch): WidgetNode {
    const current = JSON.parse(JSON.stringify(root));

    // Handle move: extract then re-insert
    if (patch.move) {
      let movedNode: any = null;
      function extract(node: any) {
        if (Array.isArray(node.children)) {
          const idx = node.children.findIndex((c: any) => c.id === patch.move!.id);
          if (idx !== -1) {
            movedNode = node.children.splice(idx, 1)[0];
            return;
          }
          for (const child of node.children) extract(child);
        }
      }
      extract(current);
      if (movedNode) {
        function insertInto(node: any) {
          if (node.id === patch.move!.newParentId && Array.isArray(node.children)) {
            const idx = patch.move!.newIndex !== undefined ? patch.move!.newIndex : node.children.length;
            node.children.splice(idx, 0, movedNode);
            return;
          }
          if (Array.isArray(node.children)) {
            for (const child of node.children) insertInto(child);
          }
        }
        insertInto(current);
      }
    }

    function visit(node: any) {
      if (!node) return;
      const id = node.id;
      if (id) {
        if (patch.visibility && patch.visibility[id] !== undefined) {
          node.hidden = !patch.visibility[id];
        }
        if (patch.setProps && patch.setProps[id]) {
          Object.assign(node, patch.setProps[id]);
        }
        if (patch.insert && patch.insert.parentId === id && Array.isArray(node.children)) {
          const idx = patch.insert.index !== undefined ? patch.insert.index : node.children.length;
          node.children.splice(idx, 0, patch.insert.node);
        }
      }

      if (Array.isArray(node.children)) {
        if (patch.remove) {
          node.children = node.children.filter((c: any) => c.id !== patch.remove!.id);
        }
        for (const child of node.children) {
          visit(child);
        }
      }
    }

    visit(current);
    return current;
  }

  export function applyPatch(patch: PluginPatch): void {
    if (patch.values) {
      for (const [k, v] of Object.entries(patch.values)) {
        values[k] = v;
      }
    }
    if (uiRoot) {
      uiRoot = applyPatchToTree(uiRoot, patch);
      collectDefaults(uiRoot);
    }
  }

  async function loadScreen(targetScreen = currentScreen): Promise<boolean> {
    if (!pluginId || loading || actionLoading !== null) return false;
    loading = true;
    error = null;
    try {
      const res = await pluginRender(pluginId, targetScreen, {}, get(currentLanguage), uiMode.expertMode);
      triggerPluginSnackbar(res);
      if (!res?.root) return false;
      currentScreen = targetScreen;
      uiRoot = res.root;
      collectDefaults(uiRoot);
      return true;
    } catch (e: any) {
      error = e?.message || String(e);
      if (uiRoot) snackbar.show(error ?? "", "error");
      return false;
    } finally {
      loading = false;
    }
  }

  async function goBack() {
    if (loading || actionLoading !== null) return;
    const previous = previousScreens.at(-1);
    if (!previous) {
      onExit?.();
    } else if (await loadScreen(previous)) {
      previousScreens = previousScreens.slice(0, -1);
    }
  }

  function inTitleBar(node: WidgetNode): boolean {
    return node.type === "button" && (node.placement === "header" || (!node.placement && node.variant === "settings"));
  }

  function headerButtons(node: WidgetNode | null): ButtonWidget[] {
    if (!node || node.hidden || (node.expertOnly && !uiMode.expertMode)) return [];
    if (node.type === "button" && inTitleBar(node)) return [node];
    return "children" in node ? node.children.flatMap(headerButtons) : [];
  }

  function findWidget(node: WidgetNode | null, id: string): WidgetNode | undefined {
    if (node?.id === id) return node;
    if (node && "children" in node) {
      for (const child of node.children) {
        const match = findWidget(child, id);
        if (match) return match;
      }
    }
  }

  function findButton(node: WidgetNode | null, id: string): ButtonWidget | undefined {
    const match = findWidget(node, id);
    if (match?.type === "button") return match;
  }

  function hasFileDrop(node: WidgetNode | null): boolean {
    if (!node) return false;
    if (node.type === "pathInput") return node.pickerType === "file" && node.drop === true;
    return "children" in node && node.children.some(hasFileDrop);
  }

  const acceptsFileDrop = $derived(hasFileDrop(uiRoot));
  $effect(() => {
    if (!active || !acceptsFileDrop) return;
    return setupWebviewDragDrop({
      isActive: () => active,
      setDraggingOver: () => {},
      onDrop(paths, position) {
        if (!position || paths.length !== 1 || actionLoading !== null) return;
        const target = document.elementFromPoint(position.x, position.y)?.closest<HTMLElement>("[data-plugin-file-drop]");
        if (!target || !renderer?.contains(target)) return;
        const node = findWidget(uiRoot, target.dataset.pluginFileDrop ?? "");
        if (node?.type !== "pathInput" || !node.drop || node.disabled || node.pickerType !== "file") return;
        const path = paths[0];
        const extensions = node.filters?.flatMap(filter => filter.extensions) ?? [];
        if (extensions.length && !extensions.some(ext => ext === "*" || path.toLowerCase().endsWith("." + ext.toLowerCase()))) return;
        values[node.id] = path;
        void handleAction(node.id, "change");
      },
    });
  });

  function missingRequiredInput(node: ButtonWidget): boolean {
    return !!node.requiresAny?.length && !node.requiresAny.some(id => {
      const value = values[id];
      return typeof value === "string" ? value.trim().length > 0 : value != null && value !== false;
    });
  }

  function pulseOutline(target: HTMLElement) {
    if (!target.animate || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    for (const animation of target.getAnimations?.() ?? []) animation.cancel();
    target.animate(
      [{ boxShadow: "0 0 0 0 transparent" }, { boxShadow: "0 0 0 3px var(--color-amber-400)" }, { boxShadow: "0 0 0 0 transparent" }],
      { duration: 420, iterations: 2 },
    );
  }

  function pulseMissingPassword(target: HTMLElement, warning: string | undefined) {
    if (warning) pulseOutline(target);
  }

  function activateButton(node: ButtonWidget) {
    if (loading || actionLoading !== null) return;
    if ((node.disabled || missingRequiredInput(node)) && node.disabledFocus) {
      const target = Array.from(renderer.querySelectorAll<HTMLElement>("[id]"))
        .find(element => element.id === `widget-${node.disabledFocus}`);
      if (target) {
        target.focus();
        pulseOutline(target);
      }
      return;
    }
    void handleAction(node.id);
  }

  export async function handleAction(widgetId: string, eventType: string = "click"): Promise<void> {
    if (loading || actionLoading !== null) return;
    const button = findButton(uiRoot, widgetId);
    if (button && (button.disabled || missingRequiredInput(button))) return;
    actionLoading = widgetId;
    error = null;
    try {
      const res = await pluginDispatchEvent(
        pluginId,
        {
          screen: currentScreen,
          widgetId,
          eventType,
          values: $state.snapshot(values),
          expertMode: uiMode.expertMode,
        },
        get(currentLanguage),
        uiMode.expertMode
      );

      if (res) {
        triggerPluginSnackbar(res);
        if (res.root) {
          if (typeof res.screen === "string" && res.screen.trim() && res.screen !== currentScreen) {
            previousScreens = [...previousScreens, currentScreen];
            currentScreen = res.screen;
          }
          uiRoot = res.root;
          collectDefaults(uiRoot);
        }
        if (res.patch) {
          applyPatch(res.root ? { values: res.patch.values } : res.patch);
        }
      }
    } catch (e: any) {
      const msg = e?.message || String(e);
      error = msg;
      snackbar.show(msg, "error");
    } finally {
      actionLoading = null;
    }
  }

  async function handleSaveSettings(fields: PluginSettingField[]): Promise<void> {
    const subset: Record<string, any> = {};
    for (const f of fields) {
      if (values[f.id] !== undefined) {
        subset[f.id] = values[f.id];
      }
    }
    try {
      await pluginSaveSettings(pluginId, subset);
      snackbar.show(t("plugins.settingsSaved"), "success");
    } catch (e: any) {
      snackbar.show(e?.message || String(e), "error");
    }
  }

  async function copyPath(path: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(path);
      snackbar.show(t("common.copied"), "success");
    } catch {
      snackbar.show(path, "info");
    }
  }

  async function openPath(path: string): Promise<void> {
    try {
      await invokeCommand("open_output_path", { path, folder: false });
    } catch (e: any) {
      snackbar.show(e?.message || String(e), "error");
    }
  }

  $effect(() => {
    if (active && pluginId && !initialRoot) {
      untrack(() => { if (!uiRoot) void loadScreen(); });
    }
  });

  onMount(() => {
    const handleUpdate = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (
        detail &&
        detail.pluginId === pluginId &&
        (!detail.screen || detail.screen === currentScreen)
      ) {
        if (detail.root) {
          uiRoot = detail.root;
          collectDefaults(uiRoot);
        }
        if (detail.patch) {
          applyPatch(detail.root ? { values: detail.patch.values } : detail.patch);
        }
      }
    };

    window.addEventListener("vesta-plugin-ui-update", handleUpdate);
    return () => {
      window.removeEventListener("vesta-plugin-ui-update", handleUpdate);
    };
  });

  function formatGap(gap?: number | string): string {
    if (gap === undefined || gap === null) return "0.75rem";
    if (typeof gap === "number") return `${gap * 0.25}rem`;
    return gap;
  }

  function getRowAlign(align?: string): string {
    if (align === "start") return "items-start justify-start";
    if (align === "center") return "items-center justify-start";
    if (align === "end") return "items-end justify-start";
    if (align === "between") return "items-center justify-between";
    return "items-center";
  }

  function getTextVariantClass(variant?: string): string {
    if (variant === "muted") return "text-gray-400";
    if (variant === "danger") return "text-rose-400";
    if (variant === "success") return "text-emerald-400";
    return "text-gray-200";
  }

  function getTextSizeClass(size?: string): string {
    if (size === "sm") return "text-xs";
    if (size === "lg") return "text-lg";
    return "text-sm";
  }

  async function handleBrowsePath(node: PathInputWidget): Promise<void> {
    const isDirectory = node.pickerType !== "file";
    try {
      const selected = await guardedOpen({
        directory: isDirectory,
        multiple: false,
        filters: node.filters,
      });
      if (selected && typeof selected === "string") {
        values[node.id] = selected;
        void handleAction(node.id, "change");
      }
    } catch (e: any) {
      snackbar.show(e?.message || String(e), "error");
    }
  }

  interface ResolvedHintAndLink {
    text: string;
    link?: { label: string; url: string };
  }

  function resolveHintAndLink(
    hint?: string | { text?: string; link?: { label: string; url: string } },
    helpText?: string,
    link?: { label: string; url: string }
  ): ResolvedHintAndLink {
    let rawText = "";
    let extractedLink = link;

    if (typeof hint === "string") {
      rawText = hint;
    } else if (hint && typeof hint === "object") {
      rawText = hint.text ?? "";
      if (!extractedLink && hint.link) {
        extractedLink = hint.link;
      }
    } else if (helpText) {
      rawText = helpText;
    }

    // Check if rawText contains markdown link syntax: [label](url)
    const mdMatch = rawText.match(/\[([^\]]+)\]\((https?:\/\/[^\)]+)\)/);
    if (mdMatch) {
      if (!extractedLink) {
        extractedLink = { label: mdMatch[1], url: mdMatch[2] };
      }
      rawText = rawText.replace(mdMatch[0], "").trim();
    }

    // Deduplicate redundant bracketed label if extracted link matches
    if (extractedLink && extractedLink.label) {
      rawText = rawText.replace(`[${extractedLink.label}]`, "").trim();
    }

    return { text: rawText, link: extractedLink };
  }

  function getButtonVariantClass(variant?: string): string {
    if (variant === "settings") return "plugin-settings-button plugin-action";
    if (variant === "accent") return "plugin-action plugin-action-accent";
    if (variant === "secondary") return "btn-secondary btn-compact";
    if (variant === "danger") return "plugin-action plugin-action-danger";
    return "plugin-action plugin-action-primary";
  }
</script>

{#snippet renderIcon(name: string, cls = "w-4 h-4")}
  {#if name === "clear"}
    <svg class={cls} aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-width="2" d="m6 6 12 12M18 6 6 18"/></svg>
  {:else if name === "trash"}
    <svg class={cls} aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7h16M9 7V4h6v3m3 0-1 13H7L6 7m4 4v5m4-5v5"/></svg>
  {:else if name === "save"}
    <svg class={cls} aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linejoin="round" stroke-width="2" d="M5 3h12l4 4v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm2 0v6h10V3M7 21v-8h10v8"/></svg>
  {:else if name === "search"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
  {:else if name === "folder"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
  {:else if name === "file"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>
  {:else if name === "film"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
  {:else if name === "subtitles"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 8h10M7 12h4m1 0h5M7 16h8M5 4h14a2 2 0 012 2v12a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2z"/></svg>
  {:else if name === "settings"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
  {:else if name === "download"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/></svg>
  {:else if name === "sparkles" || name === "cards"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/></svg>
  {:else if name === "info"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
  {:else if name === "check"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
  {:else if name === "refresh"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
  {:else if name === "trash"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
  {:else if name === "external"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
  {:else if name === "key"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/></svg>
  {:else if name === "globe"}
    <svg class={cls} fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/></svg>
  {:else if name === "tmdb"}
    <svg class="h-4 w-auto shrink-0" viewBox="0 0 190 138" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="tmdb-grad" y1="0" x1="0" y2="138" x2="190" gradientUnits="userSpaceOnUse">
          <stop stop-color="#90cea1" offset="0"/>
          <stop stop-color="#3cbec9" offset="0.56"/>
          <stop stop-color="#00b3e5" offset="1"/>
        </linearGradient>
      </defs>
      <rect width="190" height="138" rx="24" fill="#0d253f"/>
      <path fill="url(#tmdb-grad)" d="M26 30h34v10H48v32H38V40H26V30zm40 0h12l10 22 10-22h12v42h-10V47l-8 18h-8l-8-18v25H66V30zm54 0h18c12 0 20 8 20 21s-8 21-20 21h-18V30zm10 10v22h8c6 0 10-4 10-11s-4-11-10-11h-8zm34-10h20a10 10 0 010 20h-20V30zm0 82h-22a10 10 0 010-20h22a10 10 0 010 20zm-58 0H26V76h18c11 0 18 7 18 18s-7 18-18 18zm-8-26v16h8c5 0 8-3 8-8s-3-8-8-8h-8zm36-10h20c7 0 12 4 12 10 0 4-2 7-6 8 5 1 8 4 8 9 0 6-5 9-13 9H74V76zm10 8v6h9c2 0 4-1 4-3s-2-3-4-3h-9zm0 14v6h10c3 0 4-1 4-3s-1-3-4-3H84z"/>
    </svg>
  {:else if name === "imdb"}
    <svg class="h-4 w-auto shrink-0" viewBox="0 0 64 32" xmlns="http://www.w3.org/2000/svg">
      <rect width="64" height="32" rx="5" fill="#F5C518"/>
      <path fill="#000000" d="M8 7h5v18H8V7zm9 0h6.5l2.5 11 2.5-11H35v18h-4.2V13l-3 12h-3.6l-3-12v12H17V7zm21 0h6.8c4.4 0 6.7 2.5 6.7 6.8v4.4c0 4.3-2.3 6.8-6.7 6.8H38V7zm4.8 3.6v10.8h1.6c1.6 0 2.3-.9 2.3-2.9v-5c0-2-.7-2.9-2.3-2.9h-1.6zM53 7h4.4v5.8c.7-.9 1.6-1.4 2.7-1.4 2.2 0 3.4 1.6 3.4 4.5v4.7c0 2.9-1.2 4.6-3.4 4.6-1.2 0-2.1-.5-2.8-1.5V25H53V7zm4.4 8.5v5.2c0 1 .4 1.5 1.1 1.5s1.1-.5 1.1-1.5v-5.2c0-1-.4-1.5-1.1-1.5s-1.1.5-1.1 1.5z"/>
    </svg>
  {/if}
{/snippet}

{#snippet renderNode(node: WidgetNode, inHeader = false)}
  {#if !node.hidden && (!node.expertOnly || uiMode.expertMode) && (!header || !inTitleBar(node) || inHeader)}
    {#if node.type === "column"}
      <div class="flex flex-col w-full" style="gap: {formatGap(node.gap)}">
        {#each node.children as child}
          {@render renderNode(child)}
        {/each}
      </div>

    {:else if node.type === "row"}
      {@const children = node.children.filter(child => !child.hidden && (!child.expertOnly || uiMode.expertMode) && (!header || !inTitleBar(child)))}
      <div class="flex flex-row flex-wrap w-full {getRowAlign(node.align)}" style="gap: {formatGap(node.gap)}">
        {#each children as child, index}
          {#if index > 0 && node.separator === "or"}
            <span class="self-center text-sm" style:color="var(--muted)">{t("common.or")}</span>
          {/if}
          <div
            class={child.width && typeof child.width === 'string' && child.width.startsWith('w-') ? child.width : ''}
            style="{child.flex !== undefined ? `flex: ${child.flex} ${child.flex} 0px; min-width: 0;` : ''}{child.width && typeof child.width === 'number' ? `width: ${child.width}px;` : child.width && typeof child.width === 'string' && !child.width.startsWith('w-') ? `width: ${child.width};` : ''}"
          >
            {@render renderNode(child)}
          </div>
        {/each}
      </div>

    {:else if node.type === "text"}
      <p class="{getTextVariantClass(node.variant)} {getTextSizeClass(node.size)} leading-relaxed" class:plugin-warning-text={node.variant === "warning"}>
        {node.text}
      </p>

    {:else if node.type === "heading"}
      <div class="flex items-center gap-2">
        {#if node.icon}
          {@render renderIcon(node.icon, node.level === 1 ? "w-6 h-6 text-indigo-400 shrink-0" : "w-4 h-4 text-indigo-400 shrink-0")}
        {/if}
        {#if node.level === 1}
          <h1 class="text-2xl font-bold text-white tracking-tight">{node.text}</h1>
        {:else if node.level === 3}
          <h3 class="text-base font-semibold text-gray-200">{node.text}</h3>
        {:else if node.level === 4}
          <h4 class="text-sm font-semibold text-gray-300">{node.text}</h4>
        {:else}
          <h2 class="text-lg font-semibold text-white">{node.text}</h2>
        {/if}
      </div>

    {:else if node.type === "link" || node.type === "externalLink"}
      <a
        href={node.url}
        target="_blank"
        rel="noopener noreferrer"
        onclick={(e) => { e.preventDefault(); void openUrl(node.url); }}
        class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1.5 text-xs font-semibold transition-colors cursor-pointer"
      >
        {#if node.icon}
          {@render renderIcon(node.icon, "w-3.5 h-3.5 shrink-0")}
        {:else}
          {@render renderIcon("external", "w-3.5 h-3.5 shrink-0")}
        {/if}
        {node.label}
      </a>

    {:else if node.type === "textArea"}
      <div class="field flex flex-col gap-1.5 w-full">
        <label for="widget-{node.id}" class="text-xs font-medium text-gray-300">{node.label}</label>
        <textarea
          id="widget-{node.id}"
          bind:value={values[node.id]}
          rows={Math.min(20, Math.max(2, Math.floor(node.rows || 4)))}
          maxlength={node.maxLength}
          placeholder={node.placeholder}
          title={node.tooltip}
          disabled={node.disabled}
          readonly={node.readOnly}
          class="input-modern w-full resize-y px-3 py-2 text-sm"
        ></textarea>
      </div>

    {:else if node.type === "textInput"}
      {@const warning = !String(values[node.id] ?? "").trim() ? node.emptyWarning : undefined}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      {@const iconTooltip = node.labelTooltip || node.tooltip || (node.icon === "tmdb" || (node as any).labelIcon === "tmdb" ? "TMDB ID" : node.icon === "imdb" || (node as any).labelIcon === "imdb" ? "IMDb ID" : undefined)}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-300 flex items-center gap-1.5">
            {#if (node as any).labelIcon}
              <span class="inline-flex items-center shrink-0" title={iconTooltip}>
                {@render renderIcon((node as any).labelIcon, "w-4 h-4 shrink-0")}
              </span>
            {:else if node.icon === "tmdb" || node.icon === "imdb"}
              <span class="inline-flex items-center shrink-0" title={iconTooltip}>
                {@render renderIcon(node.icon, "w-4 h-4 shrink-0")}
              </span>
            {/if}
            <span>{node.label}</span>
          </label>
        {/if}
        <div class="relative w-full">
          {#if node.icon && node.icon !== "tmdb" && node.icon !== "imdb"}
            <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
              {@render renderIcon(node.icon, "w-4 h-4")}
            </div>
          {/if}
          <input
            id="widget-{node.id}"
            type="text"
            aria-describedby={warning ? `widget-${node.id}-warning` : undefined}
            class:plugin-input-warning={!!warning}
            disabled={node.disabled}
            placeholder={node.placeholder ?? ""}
            title={node.tooltip}
            bind:value={values[node.id]}
            onkeydown={(e) => {
              if (e.key === "Enter" && !node.disabled) {
                e.preventDefault();
                const button = node.submitAction ? findButton(uiRoot, node.submitAction) : undefined;
                if (button) activateButton(button);
                else void handleAction(node.submitAction || node.id, node.submitAction ? "click" : "submit");
              }
            }}
            class="h-10 px-3 {node.icon && node.icon !== 'tmdb' && node.icon !== 'imdb' ? 'pl-9' : ''} rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50 w-full"
          />
        </div>
        {#if warning}
          <span id="widget-{node.id}-warning" class="text-xs plugin-warning-text">{warning}</span>
        {/if}
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "passwordInput"}
      {@const warning = !String(values[node.id] ?? "").trim() ? node.emptyWarning : undefined}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      {@const iconTooltip = node.labelTooltip || node.tooltip || (node.icon === "tmdb" || (node as any).labelIcon === "tmdb" ? "TMDB ID" : node.icon === "imdb" || (node as any).labelIcon === "imdb" ? "IMDb ID" : undefined)}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-300 flex items-center gap-1.5">
            {#if (node as any).labelIcon}
              <span class="inline-flex items-center shrink-0" title={iconTooltip}>
                {@render renderIcon((node as any).labelIcon, "w-4 h-4 shrink-0")}
              </span>
            {:else if node.icon === "tmdb" || node.icon === "imdb"}
              <span class="inline-flex items-center shrink-0" title={iconTooltip}>
                {@render renderIcon(node.icon, "w-4 h-4 shrink-0")}
              </span>
            {/if}
            <span>{node.label}</span>
            {#if warning}
              <span aria-hidden="true" class="plugin-field-dot" title={warning}></span>
            {/if}
          </label>
        {/if}
        <div class="relative w-full">
          {#if node.icon && node.icon !== "tmdb" && node.icon !== "imdb"}
            <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-gray-400">
              {@render renderIcon(node.icon, "w-4 h-4")}
            </div>
          {/if}
          <input
            id="widget-{node.id}"
            type="password"
            use:pulseMissingPassword={warning}
            aria-describedby={warning ? `widget-${node.id}-warning` : undefined}
            class:plugin-input-warning={!!warning}
            disabled={node.disabled}
            placeholder={node.placeholder ?? ""}
            title={node.tooltip}
            bind:value={values[node.id]}
            onkeydown={(e) => {
              if (e.key === "Enter" && !node.disabled) {
                e.preventDefault();
                const button = node.submitAction ? findButton(uiRoot, node.submitAction) : undefined;
                if (button) activateButton(button);
                else void handleAction(node.submitAction || node.id, node.submitAction ? "click" : "submit");
              }
            }}
            class="h-10 px-3 {node.icon && node.icon !== 'tmdb' && node.icon !== 'imdb' ? 'pl-9' : ''} rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50 w-full"
          />
        </div>
        {#if warning}
          <span id="widget-{node.id}-warning" class="sr-only">{warning}</span>
        {/if}
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "numberInput"}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-400">
            {node.label}
          </label>
        {/if}
        <input
          id="widget-{node.id}"
          type="number"
          min={node.min}
          max={node.max}
          step={node.step}
          disabled={node.disabled}
          bind:value={values[node.id]}
          class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50"
        />
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "select"}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-400">
            {node.label}
          </label>
        {/if}
        <div class="relative">
          <select
            id="widget-{node.id}"
            disabled={node.disabled}
            bind:value={values[node.id]}
            class="h-10 w-full px-3 pr-8 rounded-xl border border-white/10 bg-gray-800 text-sm text-gray-100 appearance-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50 cursor-pointer"
          >
            {#each node.options as opt}
              <option value={opt.value} class="bg-gray-800 text-gray-100">{opt.label}</option>
            {/each}
          </select>
          <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2.5 text-gray-400">
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
          </div>
        </div>
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "segmented"}
      <div class="flex flex-col gap-1.5">
        {#if node.label}
          <span class="text-xs font-medium text-gray-400">{node.label}</span>
        {/if}
        <div class="inline-flex p-1 rounded-xl bg-black/40 border border-white/10 gap-1 self-start">
          {#each node.options as opt}
            {@const selected = (values[node.id] ?? node.defaultValue) === opt.value}
            <button
              type="button"
              disabled={node.disabled}
              title={opt.tooltip}
              class="px-3 py-1.5 rounded-lg text-xs font-medium transition-all inline-flex items-center gap-1.5 cursor-pointer {selected ? 'bg-indigo-600 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'} disabled:opacity-50"
              onclick={() => {
                values[node.id] = opt.value;
                void handleAction(node.id, "change");
              }}
            >
              {#if opt.icon}
                {@render renderIcon(opt.icon, "w-3.5 h-3.5 shrink-0")}
              {/if}
              <span>{opt.label}</span>
              {#if opt.badge}
                <span class="ml-1 px-1.5 py-0.5 rounded text-[10px] {selected ? 'bg-white/20 text-white' : 'bg-white/10 text-gray-400'}">
                  {opt.badge}
                </span>
              {/if}
            </button>
          {/each}
        </div>
      </div>

    {:else if node.type === "pathInput"}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      {@const droppable = node.pickerType === "file" && node.drop && !node.disabled}
      <div
        class="field flex flex-col gap-1 w-full"
        class:plugin-file-drop={droppable}
        data-plugin-file-drop={droppable ? node.id : undefined}
      >
        <PathPickerField
          label={node.label ?? ""}
          value={values[node.id] ?? ""}
          placeholder={node.placeholder ?? t(node.pickerType === "file" ? "plugins.chooseFile" : "plugins.chooseFolder")}
          browseTitle={t("common.browse")}
          disabled={node.disabled}
          browseFirst={node.pickerType !== "file"}
          clearLabel={node.pickerType !== "file" ? t("common.clearField") : undefined}
          onbrowse={() => handleBrowsePath(node)}
          onclear={() => {
            values[node.id] = "";
            void handleAction(node.id, "change");
          }}
        />
        {#if droppable}
          <p class="text-xs text-gray-400">{node.dropHint ?? t("sync.dropFileHere")}</p>
        {/if}
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "languageSelect"}
      {@const info = resolveHintAndLink(node.hint, node.helpText, node.link)}
      {@const catalog = node.scope === "core" ? languages : extendedLanguages}
      {@const available = node.codes ? [...new Set(node.codes)].flatMap(code => catalog.filter(language => language.code === code)) : catalog}
      <div class="field flex flex-col gap-1.5 w-full" title={node.tooltip}>
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-400">
            {node.label}
          </label>
        {/if}
        <SearchableSelect
          options={available.map(language => ({
            value: language.code,
            label: node.names === "english" ? language.nameEn : node.names === "native" || language.nameEn === language.name
              ? language.name : `${language.nameEn} (${language.name})`,
            searchTerms: `${language.nameEn} ${language.name} ${language.code}`,
            icon: node.showFlags === false ? undefined : language.flag,
          }))}
          value={values[node.id] ?? node.defaultValue ?? "en"}
          onchange={(val) => {
            values[node.id] = val;
            void handleAction(node.id, "change");
          }}
          disabled={node.disabled}
          placeholder={node.placeholder ?? t("flashcards.noteTypeLanguagePlaceholder")}
          ariaLabel={node.label}
          portal={true}
        />
        {#if info.text || info.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if info.text}
              <span class="text-xs text-gray-500">{info.text}</span>
            {/if}
            {#if info.link}
              <a
                href={info.link.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                {@render renderIcon("external", "w-3 h-3")}
                {info.link.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "checkbox"}
      <button
        type="button"
        role="checkbox"
        aria-checked={Boolean(values[node.id])}
        disabled={node.disabled}
        title={node.tooltip}
        class="flex items-center gap-3 p-3 rounded-xl border transition-all text-left cursor-pointer {Boolean(values[node.id]) ? 'border-indigo-500/50 bg-indigo-500/10 text-white' : 'border-white/10 bg-white/[0.03] text-gray-300 hover:bg-white/[0.06]'} disabled:opacity-50"
        onclick={() => { if (!node.disabled) values[node.id] = !values[node.id]; }}
        onkeydown={(e) => { if ((e.key === ' ' || e.key === 'Enter') && !node.disabled) { e.preventDefault(); values[node.id] = !values[node.id]; } }}
      >
        <div class="w-5 h-5 rounded-lg border flex items-center justify-center transition-colors {Boolean(values[node.id]) ? 'border-indigo-400 bg-indigo-600 text-white' : 'border-white/20 bg-black/20 text-transparent'}">
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/></svg>
        </div>
        <span class="text-sm font-medium">{node.label}</span>
      </button>

    {:else if node.type === "button"}
      {@const blocked = !!node.disabled || missingRequiredInput(node) || loading || actionLoading !== null}
      {@const guidance = node.disabled ? node.disabledTooltip ?? node.tooltip : node.tooltip}
      <button
        type="button"
        id="widget-{node.id}"
        data-plugin-button={node.id}
        disabled={blocked && !(!loading && actionLoading === null && node.disabledFocus)}
        aria-disabled={blocked || undefined}
        title={guidance}
        class="relative inline-flex items-center justify-center gap-1.5 font-medium h-10 px-4 text-xs rounded-xl transition-colors disabled:opacity-50 aria-disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed {getButtonVariantClass(node.variant)}"
        onclick={() => activateButton(node)}
      >
        {#if actionLoading === node.id}
          <svg class="animate-spin -ml-1 mr-1.5 h-3.5 w-3.5 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
        {:else if node.icon}
          {@render renderIcon(node.icon, "w-3.5 h-3.5 shrink-0 " + (node.icon === "save" || node.icon === "refresh" ? "text-white" : ""))}
        {/if}
        {#if node.status === "attention" || node.status === "ready"}
          <span aria-hidden="true" class="plugin-status-dot" class:attention={node.status === "attention"}></span>
        {/if}
        <span>{node.label}</span>
      </button>

    {:else if node.type === "list"}
      <div class="flex flex-col gap-2 w-full">
        {#each node.items as item (item.id)}
          {@const primary = item.actions?.find(action => action.id === item.primaryAction)}
          <div class="relative flex items-center justify-between p-3.5 rounded-xl border border-white/10 bg-white/[0.03] hover:bg-white/[0.05] transition-colors gap-3">
            {#if primary}
              <button type="button" class="absolute inset-0 rounded-xl cursor-pointer focus-visible:outline-2"
                aria-label={item.title} disabled={actionLoading !== null}
                onclick={() => handleAction(primary.id, "click")}></button>
            {/if}
            {#if item.image || node.reserveImage}
              <div class="w-12 h-12 rounded-lg bg-black/30 shrink-0 grid place-items-center overflow-hidden text-gray-500">
                {#if item.image}
                  <img src={item.image} alt="" loading="lazy" class="w-full h-full object-cover" />
                {:else}
                  {@render renderIcon("film", "w-5 h-5")}
                {/if}
              </div>
            {/if}
            <div class="flex flex-col min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-semibold text-white truncate">{item.title}</span>
                {#if item.badge}
                  <span class="px-2 py-0.5 rounded-md text-xs font-medium bg-indigo-500/20 text-indigo-300 border border-indigo-500/30 shrink-0">
                    {item.badge}
                  </span>
                {/if}
              </div>
              {#if item.language || item.subtitle || (item.meta && item.meta.length > 0)}
                {@const flagUrl = item.language ? languageFlagUrl(item.language) : null}
                {@const langName = item.language ? localizedLanguageName(item.language, $currentLanguage) : null}
                <div class="flex items-center gap-1.5 text-xs text-gray-400 mt-1 flex-wrap">
                  {#if item.language && langName}
                    <span class="inline-flex items-center gap-1.5 font-medium text-gray-200">
                      {#if flagUrl}
                        <img src={flagUrl} alt="" class="w-4 h-3 object-cover rounded-[2px] shrink-0" />
                      {/if}
                      <span>{langName}</span>
                    </span>
                    {#if item.subtitle || (item.meta && item.meta.length > 0)}
                      <span class="text-gray-600">•</span>
                    {/if}
                  {/if}
                  {#if item.meta && item.meta.length > 0}
                    <span>{item.meta.join(" • ")}</span>
                    {#if item.subtitle}
                      <span class="text-gray-600">•</span>
                    {/if}
                  {/if}
                  {#if item.subtitle}
                    <span class="truncate">{item.subtitle}</span>
                  {/if}
                </div>
              {/if}
            </div>
            {#if item.actions && item.actions.length > 0}
              <div class="flex items-center gap-2 shrink-0">
                {#each item.actions as action}
                  <button
                    type="button"
                    title={action.tooltip ?? (action.iconOnly ? action.label : undefined)}
                    aria-label={action.iconOnly ? action.label : undefined}
                    disabled={actionLoading !== null}
                    class="relative text-xs font-semibold px-3 py-1.5 rounded-lg border border-white/10 transition-colors inline-flex items-center gap-1.5 cursor-pointer {action.variant === 'primary' ? 'bg-indigo-600 hover:bg-indigo-500 text-white' : action.variant === 'danger' ? 'bg-rose-600 hover:bg-rose-500 text-white' : 'bg-white/5 hover:bg-white/10 text-gray-200'}"
                    onclick={() => handleAction(action.id, "click")}
                  >
                    {#if action.icon}
                      {@render renderIcon(action.icon, "w-3.5 h-3.5 shrink-0")}
                    {/if}
                    {#if !action.iconOnly || !action.icon}<span>{action.label}</span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>

    {:else if node.type === "table"}
      <div class="w-full overflow-x-auto rounded-xl border border-white/10">
        <table class="w-full text-left text-sm border-collapse">
          <thead class="bg-white/5 border-b border-white/10 text-xs font-semibold text-gray-300 uppercase">
            <tr>
              {#each node.headers as header}
                <th scope="col" class="px-4 py-3">{header}</th>
              {/each}
            </tr>
          </thead>
          <tbody class="divide-y divide-white/5">
            {#each node.rows as row}
              <tr class="hover:bg-white/[0.02] transition-colors">
                {#each row as cell}
                  <td class="px-4 py-3 text-gray-200">{cell}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

    {:else if node.type === "image"}
      <div class="w-full overflow-hidden rounded-xl">
        {#if node.src}
          <img src={node.src} alt={node.alt ?? ""} width={node.width} height={node.height}
            class="max-w-full rounded-xl object-contain" />
        {:else}
          <div class="grid max-w-full place-items-center rounded-xl bg-black/20 text-gray-500"
            style:width={node.width ? `${node.width}px` : undefined}
            style:height={node.height ? `${node.height}px` : undefined}>
            {@render renderIcon("film", "w-8 h-8")}
          </div>
        {/if}
      </div>

    {:else if node.type === "progress"}
      {@render progressBar(node.value ?? 0, node.max ?? 100, node.label ?? "")}

    {:else if node.type === "fileOutput"}
      <div class="flex items-center justify-between p-3 rounded-xl border border-white/10 bg-white/[0.03] gap-3 w-full">
        <div class="flex flex-col min-w-0 flex-1">
          {#if node.label}
            <span class="text-xs text-gray-400 mb-0.5">{node.label}</span>
          {/if}
          <span class="text-xs font-mono text-gray-200 truncate">{node.path}</span>
        </div>
        <div class="flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            class="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 hover:text-white transition-colors"
            title={t("plugins.copyPath")}
            aria-label={t("plugins.copyPath")}
            onclick={() => copyPath(node.path)}
          >
            {@render renderIcon("file", "w-4 h-4")}
          </button>
          <button
            type="button"
            class="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 hover:text-white transition-colors"
            title={t("common.openOutputFile")}
            aria-label={t("common.openOutputFile")}
            onclick={() => openPath(node.path)}
          >
            {@render renderIcon("external", "w-4 h-4")}
          </button>
        </div>
      </div>

    {:else if node.type === "divider"}
      <hr class="w-full border-t border-white/10 my-2" />

    {:else if node.type === "settings"}
      <div class="flex flex-col gap-4 p-4 rounded-xl border border-white/10 bg-white/[0.02] w-full">
        <h3 class="text-sm font-semibold text-white">{t("plugins.pluginSettings")}</h3>
        {#each node.fields as field}
          {#if !field.expertOnly || uiMode.expertMode}
            {#if field.type === "checkbox"}
              <button
                type="button"
                role="checkbox"
                aria-checked={Boolean(values[field.id])}
                class="flex items-center gap-3 p-3 rounded-xl border transition-all text-left cursor-pointer {Boolean(values[field.id]) ? 'border-indigo-500/50 bg-indigo-500/10 text-white' : 'border-white/10 bg-white/[0.03] text-gray-300 hover:bg-white/[0.06]'}"
                onclick={() => values[field.id] = !values[field.id]}
              >
                <div class="w-5 h-5 rounded-lg border flex items-center justify-center transition-colors {Boolean(values[field.id]) ? 'border-indigo-400 bg-indigo-600 text-white' : 'border-white/20 bg-black/20 text-transparent'}">
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"/></svg>
                </div>
                <div class="flex flex-col">
                  <span class="text-sm font-medium">{field.label}</span>
                  {#if field.description}
                    <span class="text-xs text-gray-400">{field.description}</span>
                  {/if}
                </div>
              </button>
            {:else}
              {@const info = resolveHintAndLink(field.hint, field.helpText ?? field.description, field.link)}
              <div class="flex flex-col gap-1.5 w-full">
                <label for="setting-{field.id}" class="text-xs font-medium text-gray-400">{field.label}</label>
                <input
                  id="setting-{field.id}"
                  type={field.type === "password" ? "password" : field.type === "number" ? "number" : "text"}
                  bind:value={values[field.id]}
                  placeholder={field.description ?? ""}
                  class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
                />
                {#if info.text || info.link}
                  <div class="flex flex-wrap items-center gap-2 mt-0.5">
                    {#if info.text}
                      <span class="text-xs text-gray-500">{info.text}</span>
                    {/if}
                    {#if info.link}
                      <a
                        href={info.link.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        onclick={(e) => { e.preventDefault(); void openUrl(info.link!.url); }}
                        class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-semibold transition-colors cursor-pointer"
                      >
                        {@render renderIcon("external", "w-3.5 h-3.5")}
                        {info.link.label}
                      </a>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}
          {/if}
        {/each}
        <div class="flex justify-end pt-2">
          <button
            type="button"
            class="btn-primary text-xs px-4 py-2"
            onclick={() => handleSaveSettings(node.fields)}
          >
            {t("plugins.saveSettings")}
          </button>
        </div>
      </div>
    {/if}
  {/if}
{/snippet}

{#snippet progressBar(value: number, max = 100, label = "", showHeaderWithoutLabel = false, progressId?: string)}
  {@const safeMax = max > 0 ? max : 100}
  {@const ratio = Math.min(100, Math.max(0, (value / safeMax) * 100))}
  {@const percent = Math.round(ratio)}
  <div
    class="w-full flex flex-col gap-1.5"
    role="progressbar"
    aria-valuenow={percent}
    aria-valuemin={0}
    aria-valuemax={100}
    aria-label={label || progressId || undefined}
    data-plugin-progress={progressId}
  >
    {#if label || showHeaderWithoutLabel}
      <div class="flex justify-between text-xs text-gray-400">
        <span>{label}</span>
        <span>{percent}%</span>
      </div>
    {/if}
    <div class="w-full h-2 rounded-full bg-white/10 overflow-hidden">
      <div
        class="h-full bg-indigo-500 rounded-full transition-all duration-200"
        style="width: {ratio}%"
      ></div>
    </div>
  </div>
{/snippet}

{#snippet actions()}
  {#each headerButtons(uiRoot) as button (button.id)}
    {@render renderNode(button, true)}
  {/each}
{/snippet}

<div bind:this={renderer} class="w-full h-full flex flex-col">
  {@render header?.(actions, () => { void goBack(); }, loading || actionLoading !== null)}
  <div class="plugin-renderer w-full flex-1 min-h-0 flex flex-col p-6 overflow-y-auto">
    {#if activeProgress.length > 0}
      <div class="mb-4 flex flex-col gap-2 shrink-0" data-testid="plugin-active-progress">
        {#each activeProgress as entry (entry.id)}
          {@render progressBar(entry.value, 100, entry.label, true, entry.id)}
        {/each}
      </div>
    {/if}
    {#if loading && !uiRoot}
      <div class="flex-1 flex flex-col items-center justify-center gap-3 text-sm text-gray-400">
        <div class="skeleton w-48 h-6 rounded-lg"></div>
        <div class="skeleton w-72 h-10 rounded-xl"></div>
        <div class="skeleton w-64 h-32 rounded-xl"></div>
      </div>
    {:else if error && !uiRoot}
      <div class="flex-1 flex flex-col items-center justify-center gap-3 text-center p-6">
        <div class="w-12 h-12 rounded-2xl bg-rose-500/10 border border-rose-500/20 text-rose-400 flex items-center justify-center">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
        </div>
        <p class="text-sm font-semibold text-rose-400">{error}</p>
        <button class="btn-secondary text-xs px-4 py-2 mt-2" onclick={() => loadScreen()}>
          {t("common.retry")}
        </button>
      </div>
    {:else if uiRoot}
      {@render renderNode(uiRoot)}
    {:else}
      <div class="flex-1 flex flex-col items-center justify-center gap-2 text-gray-500 text-sm">
        <p>{t("plugins.emptyScreen")}</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .plugin-action {
    background: var(--control-bg);
    border: 1px solid var(--control-border);
  }
  .plugin-action:hover:not(:disabled):not([aria-disabled="true"]) {
    background: var(--control-focus-bg);
  }
  .plugin-action-primary {
    color: var(--text);
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .plugin-action-primary:hover:not(:disabled):not([aria-disabled="true"]) {
    background: var(--accent-strong);
  }
  .plugin-action-accent {
    --action-fill: var(--color-cyan-600);
    color: var(--text);
  }
  .plugin-action-danger {
    --action-fill: var(--color-rose-700);
    color: var(--text);
  }
  :is(.plugin-action-accent, .plugin-action-danger) {
    background: color-mix(in srgb, var(--action-fill) 78%, var(--surface-2));
    border-color: color-mix(in srgb, var(--action-fill) 90%, var(--surface-2));
  }
  :is(.plugin-action-accent, .plugin-action-danger):hover:not(:disabled):not([aria-disabled="true"]) {
    background: color-mix(in srgb, var(--action-fill) 90%, var(--surface-2));
  }
  .plugin-action:disabled {
    background: var(--control-bg);
    border-color: var(--control-border);
    color: var(--muted);
  }
  .plugin-settings-button { color: var(--text); }
  .plugin-settings-button:focus-visible {
    outline: 2px solid var(--color-amber-400);
    outline-offset: 3px;
  }
  .plugin-file-drop {
    border: 1px dashed var(--control-border);
    border-radius: 0.75rem;
    padding: 0.75rem;
  }

  .plugin-status-dot {
    position: absolute;
    inset-block-start: -0.12rem;
    inset-inline-end: -0.12rem;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--bg);
    inline-size: 0.45rem;
    block-size: 0.45rem;
    border-radius: 50%;
    background: var(--done);
  }
  .plugin-field-dot {
    flex: none;
    inline-size: 0.4rem;
    block-size: 0.4rem;
    border-radius: 50%;
    background: var(--color-amber-400);
  }
  .plugin-input-warning {
    border-color: var(--color-amber-400);
  }
  .plugin-warning-text {
    color: var(--color-amber-400);
  }
  .plugin-status-dot.attention {
    background: var(--color-amber-400);
  }
</style>

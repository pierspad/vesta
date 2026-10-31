<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    pluginRender,
    pluginDispatchEvent,
    pluginSaveSettings,
    type WidgetNode,
    type PluginSettingField,
    type PluginPatch,
  } from "$lib/services/pluginClient";
  import { currentLanguage, locale } from "$lib/i18n";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { invokeCommand } from "$lib/services/tauriClient";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";

  interface Props {
    pluginId: string;
    screen?: string;
    active?: boolean;
    initialRoot?: WidgetNode | null;
  }

  let {
    pluginId,
    screen = "main",
    active = true,
    initialRoot = null,
  }: Props = $props();

  let t = $derived($locale);

  let uiRoot = $state<WidgetNode | null>(null);
  let values = $state<Record<string, any>>({});
  let loading = $state(false);
  let actionLoading = $state<string | null>(null);
  let error = $state<string | null>(null);

  let lastRoot: WidgetNode | null = null;
  $effect(() => {
    if (initialRoot && initialRoot !== lastRoot) {
      lastRoot = initialRoot;
      uiRoot = initialRoot;
      collectDefaults(uiRoot);
    }
  });

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
        } else if (node.type === "textInput" || node.type === "passwordInput") {
          values[id] = "";
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

  async function loadScreen(): Promise<void> {
    if (!pluginId) return;
    loading = true;
    error = null;
    try {
      const res = await pluginRender(pluginId, screen, {}, get(currentLanguage), uiMode.expertMode);
      if (res && res.root) {
        uiRoot = res.root;
        collectDefaults(uiRoot);
      } else {
        uiRoot = null;
      }
    } catch (e: any) {
      error = e?.message || String(e);
    } finally {
      loading = false;
    }
  }

  export async function handleAction(widgetId: string, eventType: string = "click"): Promise<void> {
    actionLoading = widgetId;
    error = null;
    try {
      const res = await pluginDispatchEvent(
        pluginId,
        {
          screen,
          widgetId,
          eventType,
          values: $state.snapshot(values),
          expertMode: uiMode.expertMode,
        },
        get(currentLanguage),
        uiMode.expertMode
      );

      if (res) {
        if (res.root) {
          uiRoot = res.root;
          collectDefaults(uiRoot);
        } else if (res.patch) {
          applyPatch(res.patch);
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
    if (active && pluginId && !initialRoot && !uiRoot) {
      void loadScreen();
    }
  });

  onMount(() => {
    if (initialRoot) {
      uiRoot = initialRoot;
      collectDefaults(uiRoot);
    } else if (active && pluginId) {
      void loadScreen();
    }

    const handleUpdate = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (
        detail &&
        detail.pluginId === pluginId &&
        (!detail.screen || detail.screen === screen)
      ) {
        if (detail.root) {
          uiRoot = detail.root;
          collectDefaults(uiRoot);
        } else if (detail.patch) {
          applyPatch(detail.patch);
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

  function getButtonVariantClass(variant?: string): string {
    if (variant === "secondary") return "btn-secondary";
    if (variant === "danger") return "bg-rose-600 hover:bg-rose-500 text-white font-medium px-4 py-2 rounded-xl transition-colors";
    return "btn-primary";
  }
</script>

{#snippet renderNode(node: WidgetNode)}
  {#if !node.hidden && (!node.expertOnly || uiMode.expertMode)}
    {#if node.type === "column"}
      <div class="flex flex-col w-full" style="gap: {formatGap(node.gap)}">
        {#each node.children as child}
          {@render renderNode(child)}
        {/each}
      </div>

    {:else if node.type === "row"}
      <div class="flex flex-row flex-wrap w-full {getRowAlign(node.align)}" style="gap: {formatGap(node.gap)}">
        {#each node.children as child}
          {@render renderNode(child)}
        {/each}
      </div>

    {:else if node.type === "text"}
      <p class="{getTextVariantClass(node.variant)} {getTextSizeClass(node.size)} leading-relaxed">
        {node.text}
      </p>

    {:else if node.type === "heading"}
      {#if node.level === 1}
        <h1 class="text-2xl font-bold text-white tracking-tight">{node.text}</h1>
      {:else if node.level === 3}
        <h3 class="text-base font-semibold text-gray-200">{node.text}</h3>
      {:else if node.level === 4}
        <h4 class="text-sm font-semibold text-gray-300">{node.text}</h4>
      {:else}
        <h2 class="text-lg font-semibold text-white">{node.text}</h2>
      {/if}

    {:else if node.type === "link" || node.type === "externalLink"}
      <a
        href={node.url}
        target="_blank"
        rel="noopener noreferrer"
        onclick={(e) => { e.preventDefault(); void openUrl(node.url); }}
        class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1.5 text-xs font-semibold transition-colors cursor-pointer"
      >
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
        {node.label}
      </a>

    {:else if node.type === "textInput"}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-400">
            {node.label}
          </label>
        {/if}
        <input
          id="widget-{node.id}"
          type="text"
          disabled={node.disabled}
          placeholder={node.placeholder ?? ""}
          bind:value={values[node.id]}
          class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50"
        />
        {#if node.hint || node.helpText || node.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if typeof node.hint === 'string'}
              <span class="text-xs text-gray-500">{node.hint}</span>
            {:else if typeof node.hint === 'object' && node.hint?.text}
              <span class="text-xs text-gray-500">{node.hint.text}</span>
            {:else if node.helpText}
              <span class="text-xs text-gray-500">{node.helpText}</span>
            {/if}
            {#if node.link || (typeof node.hint === 'object' && node.hint?.link)}
              {@const l = node.link || (node.hint as any).link}
              <a
                href={l.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(l.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                {l.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "passwordInput"}
      <div class="field flex flex-col gap-1.5 w-full">
        {#if node.label}
          <label for="widget-{node.id}" class="text-xs font-medium text-gray-400">
            {node.label}
          </label>
        {/if}
        <input
          id="widget-{node.id}"
          type="password"
          disabled={node.disabled}
          placeholder={node.placeholder ?? ""}
          bind:value={values[node.id]}
          class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors disabled:opacity-50"
        />
        {#if node.hint || node.helpText || node.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if typeof node.hint === 'string'}
              <span class="text-xs text-gray-500">{node.hint}</span>
            {:else if typeof node.hint === 'object' && node.hint?.text}
              <span class="text-xs text-gray-500">{node.hint.text}</span>
            {:else if node.helpText}
              <span class="text-xs text-gray-500">{node.helpText}</span>
            {/if}
            {#if node.link || (typeof node.hint === 'object' && node.hint?.link)}
              {@const l = node.link || (node.hint as any).link}
              <a
                href={l.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(l.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                {l.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "numberInput"}
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
        {#if node.hint || node.helpText || node.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if typeof node.hint === 'string'}
              <span class="text-xs text-gray-500">{node.hint}</span>
            {:else if typeof node.hint === 'object' && node.hint?.text}
              <span class="text-xs text-gray-500">{node.hint.text}</span>
            {:else if node.helpText}
              <span class="text-xs text-gray-500">{node.helpText}</span>
            {/if}
            {#if node.link || (typeof node.hint === 'object' && node.hint?.link)}
              {@const l = node.link || (node.hint as any).link}
              <a
                href={l.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(l.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                {l.label}
              </a>
            {/if}
          </div>
        {/if}
      </div>

    {:else if node.type === "select"}
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
        {#if node.hint || node.helpText || node.link}
          <div class="flex flex-wrap items-center gap-2 mt-0.5">
            {#if typeof node.hint === 'string'}
              <span class="text-xs text-gray-500">{node.hint}</span>
            {:else if typeof node.hint === 'object' && node.hint?.text}
              <span class="text-xs text-gray-500">{node.hint.text}</span>
            {:else if node.helpText}
              <span class="text-xs text-gray-500">{node.helpText}</span>
            {/if}
            {#if node.link || (typeof node.hint === 'object' && node.hint?.link)}
              {@const l = node.link || (node.hint as any).link}
              <a
                href={l.url}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => { e.preventDefault(); void openUrl(l.url); }}
                class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-medium transition-colors cursor-pointer"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                {l.label}
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
      <button
        type="button"
        id="widget-{node.id}"
        disabled={node.disabled || actionLoading === node.id}
        class="inline-flex items-center justify-center gap-2 font-medium px-4 py-2.5 rounded-xl transition-all disabled:opacity-50 cursor-pointer {getButtonVariantClass(node.variant)}"
        onclick={() => handleAction(node.id)}
      >
        {#if actionLoading === node.id}
          <svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
        {/if}
        {node.label}
      </button>

    {:else if node.type === "list"}
      <div class="flex flex-col gap-2 w-full">
        {#each node.items as item (item.id)}
          <div class="flex items-center justify-between p-3.5 rounded-xl border border-white/10 bg-white/[0.03] hover:bg-white/[0.05] transition-colors gap-3">
            {#if item.image}
              <img src={item.image} alt="" class="w-12 h-12 rounded-lg object-cover bg-black/30 shrink-0" />
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
              {#if item.subtitle}
                <span class="text-xs text-gray-400 mt-0.5 truncate">{item.subtitle}</span>
              {/if}
            </div>
            {#if item.actions && item.actions.length > 0}
              <div class="flex items-center gap-2 shrink-0">
                {#each item.actions as action}
                  <button
                    type="button"
                    class="text-xs font-semibold px-3 py-1.5 rounded-lg border border-white/10 transition-colors {action.variant === 'primary' ? 'bg-indigo-600 hover:bg-indigo-500 text-white' : action.variant === 'danger' ? 'bg-rose-600 hover:bg-rose-500 text-white' : 'bg-white/5 hover:bg-white/10 text-gray-200'}"
                    onclick={() => handleAction(node.id || item.id, action.id)}
                  >
                    {action.label}
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
        <img
          src={node.src}
          alt={node.alt ?? ""}
          width={node.width}
          height={node.height}
          class="max-w-full rounded-xl object-contain"
        />
      </div>

    {:else if node.type === "progress"}
      <div class="w-full flex flex-col gap-1.5">
        {#if node.label}
          <div class="flex justify-between text-xs text-gray-400">
            <span>{node.label}</span>
            <span>{Math.round(((node.value ?? 0) / (node.max ?? 100)) * 100)}%</span>
          </div>
        {/if}
        <div class="w-full h-2 rounded-full bg-white/10 overflow-hidden">
          <div
            class="h-full bg-indigo-500 rounded-full transition-all duration-200"
            style="width: {Math.min(100, Math.max(0, ((node.value ?? 0) / (node.max ?? 100)) * 100))}%"
          ></div>
        </div>
      </div>

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
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
          </button>
          <button
            type="button"
            class="p-2 rounded-lg bg-white/5 hover:bg-white/10 text-gray-300 hover:text-white transition-colors"
            title={t("common.openOutputFile")}
            aria-label={t("common.openOutputFile")}
            onclick={() => openPath(node.path)}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
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
              <div class="flex flex-col gap-1.5 w-full">
                <label for="setting-{field.id}" class="text-xs font-medium text-gray-400">{field.label}</label>
                <input
                  id="setting-{field.id}"
                  type={field.type === "password" ? "password" : field.type === "number" ? "number" : "text"}
                  bind:value={values[field.id]}
                  placeholder={field.description ?? ""}
                  class="h-10 px-3 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
                />
                <div class="flex flex-wrap items-center gap-2 mt-0.5">
                  {#if field.description}
                    <span class="text-xs text-gray-500">{field.description}</span>
                  {:else if field.helpText}
                    <span class="text-xs text-gray-500">{field.helpText}</span>
                  {:else if typeof field.hint === 'string'}
                    <span class="text-xs text-gray-500">{field.hint}</span>
                  {/if}
                  {#if field.link || (typeof field.hint === 'object' && field.hint?.link)}
                    {@const l = field.link || (field.hint as any).link}
                    <a
                      href={l.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      onclick={(e) => { e.preventDefault(); void openUrl(l.url); }}
                      class="text-indigo-400 hover:text-indigo-300 hover:underline inline-flex items-center gap-1 text-xs font-semibold transition-colors cursor-pointer"
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                      {l.label}
                    </a>
                  {/if}
                </div>
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

<div class="plugin-renderer w-full h-full flex flex-col p-6 overflow-y-auto">
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
      <button class="btn-secondary text-xs px-4 py-2 mt-2" onclick={loadScreen}>
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

<script lang="ts">
  import { githubReleaseEndpoint, newerPluginRelease } from "$lib/services/pluginReleases";
  import { updateCheckerStore } from "$lib/stores/updateCheckerStore.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { onMount, untrack } from "svelte";
  import { pluginsStore } from "$lib/stores/pluginsStore.svelte";
  import { locale } from "$lib/i18n";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PluginRenderer from "$lib/components/PluginRenderer.svelte";
  import MarkdownContent from "$lib/components/MarkdownContent.svelte";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import SearchInput from "$lib/components/SearchInput.svelte";
  import { uiMode } from "$lib/stores/uiModeStore.svelte";
  import type { InstalledPlugin, PluginInspectResult, PluginManifest } from "$lib/services/pluginClient";

  interface Props {
    active?: boolean;
    activePluginId?: string | null;
    onGoToTab?: (tab: string) => void;
  }

  let { active = true, activePluginId = null, onGoToTab }: Props = $props();

  let t = $derived($locale);

  let selectedPluginId = $state<string | null>(null);

  $effect(() => {
    if (activePluginId !== undefined) {
      selectedPluginId = activePluginId;
    }
  });

  let currentPlugin = $derived(
    selectedPluginId ? pluginsStore.plugins.find((p) => p.id === selectedPluginId) ?? null : null
  );

  let installMode = $state<"repo" | "folder">("repo");
  let urlInput = $state("");
  let inspectResult = $state<PluginInspectResult | null>(null);
  let inspecting = $state(false);
  let installing = $state(false);
  let searchQuery = $state("");

  let uninstallTarget = $state<InstalledPlugin | null>(null);
  let settingsTarget = $state<InstalledPlugin | null>(null);
  let editSettingsValues = $state<Record<string, any>>({});

  let infoTarget = $state<InstalledPlugin | null>(null);
  let showInfoModal = $state(false);
  let showSdkModal = $state(false);
  let selectedPluginScreen = $state<string>("main");

  function getPluginImageSrc(manifest?: PluginManifest, iconUrl?: string): string | null {
    if (iconUrl) return iconUrl;
    const raw = manifest?.icon || manifest?.tab?.icon;
    if (raw && (raw.startsWith("data:image/") || raw.startsWith("https://") || raw.startsWith("http://") || raw.startsWith("/"))) {
      return raw;
    }
    return null;
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

    const mdMatch = rawText.match(/\[([^\]]+)\]\((https?:\/\/[^\)]+)\)/);
    if (mdMatch) {
      if (!extractedLink) {
        extractedLink = { label: mdMatch[1], url: mdMatch[2] };
      }
      rawText = rawText.replace(mdMatch[0], "").trim();
    }

    if (extractedLink && extractedLink.label) {
      rawText = rawText.replace(`[${extractedLink.label}]`, "").trim();
    }

    return { text: rawText, link: extractedLink };
  }

  const SDK_GUIDE_MARKDOWN = `# Vesta Plugin System & SDK Contract (v1)

Plugins in Vesta are **100% Vesta-agnostic** child processes communicating over **JSON-RPC 2.0 via standard input/output (stdio)**.

As long as Vesta honors this v1 contract, Vesta's internal GUI and Rust core can evolve freely without breaking third-party plugins.

## 1. Plugin Manifest & Visual Customization
Every plugin declares a \`vesta-plugin.json\` manifest with \`id\`, \`name\`, \`version\`, \`apiVersion: 1\`, and process execution \`entry\`.
- \`icon\`: Top-level image or icon specification. Supports named monochrome icons (\`"puzzle"\`, \`"film"\`, \`"search"\`), relative image files within the plugin repository (\`"icon.svg"\`, \`"icon.png"\`, \`"icon.webp"\`, \`"assets/logo.svg"\`, max 512 KB), or \`data:image/...\` / \`https://\` URIs.
- \`tab\`: Visual metadata for the plugin header and card (\`{ title: string, icon?: string }\`).

## 2. Host Context & Properties Provided by Vesta
On \`initialize\`, \`render\`, and \`event\`, Vesta automatically passes:
- \`apiVersion\`: \`1\`
- \`dataDir\`: Dedicated persistent directory for the plugin (\`<app_data>/plugins/installed/<id>/data\`)
- \`locale\`: Active Vesta UI language code (e.g. \`"en"\`, \`"it"\`, \`"ja"\`)
- \`expertMode\`: \`boolean\` indicating whether the user enabled **Expert Mode** in Vesta
- \`settings\`: Dictionary of saved manifest settings
- \`values\`: Snapshot of all interactive widget values on the active screen (on \`event\`)

## 3. Plugin credentials
Store credentials in the plugin's own settings or data directory. Vesta does not forward credentials between plugins.

## 4. Declarative Widget Catalog
Every widget supports \`id\`, \`hidden\`, \`expertOnly\` (auto-hidden when Expert Mode is off), \`tooltip\` (native Vesta hover tooltip), \`flex\`, and \`width\`.

- **Layout**: \`column\` (\`gap\`, \`children\`), \`row\` (\`gap\`, \`align\`: \`"start" | "center" | "end" | "between"\`, \`children\`), \`divider\`, \`heading\` (\`level\`, \`icon\`), \`text\` (\`variant\`, \`size\`)
- **Inputs**:
  - \`textInput\` / \`passwordInput\`: \`label\`, \`placeholder\`, \`defaultValue\`, \`icon\` (\`"search"\`, \`"tmdb"\`, \`"imdb"\`, etc.), \`labelTooltip\`, \`submitAction\` (button ID triggered when pressing **Enter**), \`hint\`, \`link\`
  - \`languageSelect\`: Unified Vesta language selector with bundled SVG flags (\`scope\`: \`"all"\` for 58 world languages or \`"core"\` for 33 study languages)
  - \`pathInput\`: Native OS file/folder selector (\`pickerType\`: \`"file" | "folder"\`, \`filters\`)
  - \`select\`, \`segmented\`, \`numberInput\`, \`checkbox\`
- **Actions & Display**:
  - \`button\`: \`label\`, \`variant\` (\`"primary" | "secondary" | "danger"\`), \`icon\`, \`tooltip\`
  - \`list\`: \`items\` with \`title\`, \`subtitle\`, \`badge\`, \`language\` (ISO code automatically rendered by Vesta as **Flag SVG + Localized Language Name** in Vesta's active language), \`meta\`, \`image\`, and \`actions\`
  - \`fileOutput\`, \`progress\`, \`table\`, \`image\`, \`link\` / \`externalLink\`

## 5. Host Notifications & Integrations (Plugin → Vesta)
- **Host Snackbar / Toast**: Return \`snackbar: { message: "Saved!", type: "success" | "error" | "info" | "warning" }\` in \`render\` or \`event\` response to trigger native Vesta toast notification.
- \`ui.update\`: Push live \`root\` or incremental \`patch\` (\`values\`, \`setProps\`, \`visibility\`, \`insert\`, \`remove\`, \`move\`)
- \`ui.notify\`: Show native toast (\`level\`: \`"info" | "success" | "warn" | "error"\`, \`msg\`)
- \`ui.progress\`: Update progress indicator (\`id\`, \`value\`, \`label\`)
- \`vesta.openInFlashcards\`: Hand off \`videoPath\` and \`subtitlePath\` directly to the Flashcards tab

## 6. MCP Server Support (\`vesta-mcp\`)
\`vesta-mcp\` exposes \`list_plugins\` (safe public metadata, zero secrets) and \`call_plugin\` (JSON-RPC render/event bridge) for AI agents.`;

  let contextMenuPlugin = $state<InstalledPlugin | null>(null);
  let contextMenuPos = $state<{ x: number; y: number } | null>(null);

  onMount(() => {
    if (!pluginsStore.initialized) {
      void pluginsStore.init();
    }
  });

  let filteredPlugins = $derived(
    pluginsStore.plugins.filter((p) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        p.name.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q) ||
        (p.description && p.description.toLowerCase().includes(q))
      );
    })
  );

  async function handleBrowseFolder() {
    const selected = await guardedOpen({
      directory: true,
      multiple: false,
    });
    if (selected && typeof selected === "string") {
      urlInput = selected;
    }
  }

  async function handleInspect() {
    if (!urlInput.trim()) return;
    inspecting = true;
    inspectResult = null;
    try {
      inspectResult = await pluginsStore.inspect(urlInput.trim(), undefined);
    } finally {
      inspecting = false;
    }
  }

  async function handleConfirmInstall() {
    if (!inspectResult) return;
    installing = true;
    try {
      await pluginsStore.install(urlInput.trim(), undefined);
      inspectResult = null;
      urlInput = "";
    } finally {
      installing = false;
    }
  }

  function handleCancelInspect() {
    inspectResult = null;
  }

  function handleOpenPlugin(plugin: InstalledPlugin, screen = "main") {
    if (!plugin.enabled) return;
    selectedPluginScreen = screen;
    selectedPluginId = plugin.id;
    onGoToTab?.("plugin:" + plugin.id);
  }

  function handleBackToPlugins() {
    selectedPluginId = null;
    selectedPluginScreen = "main";
    onGoToTab?.("plugins");
  }

  function openInfoModal(plugin: InstalledPlugin) {
    infoTarget = plugin;
    showInfoModal = true;
    contextMenuPlugin = null;
    contextMenuPos = null;
  }

  function openUninstallConfirm(plugin: InstalledPlugin) {
    uninstallTarget = plugin;
    contextMenuPlugin = null;
    contextMenuPos = null;
  }

  async function confirmUninstall() {
    if (!uninstallTarget) return;
    const id = uninstallTarget.id;
    if (selectedPluginId === id) {
      selectedPluginId = null;
      onGoToTab?.("plugins");
    }
    uninstallTarget = null;
    await pluginsStore.uninstall(id);
  }

  function openSettings(plugin: InstalledPlugin) {
    contextMenuPlugin = null;
    contextMenuPos = null;
    if (plugin.manifest.tab?.settingsScreen) {
      handleOpenPlugin(plugin, plugin.manifest.tab.settingsScreen);
    } else if (plugin.manifest.settings && plugin.manifest.settings.length > 0) {
      settingsTarget = plugin;
      editSettingsValues = { ...plugin.settings };
      for (const field of plugin.manifest.settings) {
        if (editSettingsValues[field.id] === undefined && field.defaultValue !== undefined) {
          editSettingsValues[field.id] = field.defaultValue;
        }
      }
    } else if (plugin.enabled) {
      handleOpenPlugin(plugin, "settings");
    }
  }

  async function savePluginSettings() {
    if (!settingsTarget) return;
    const id = settingsTarget.id;
    const values = { ...editSettingsValues };
    settingsTarget = null;
    await pluginsStore.saveSettings(id, values);
  }

  const onlinePlugins = $derived(pluginsStore.plugins.filter(plugin => plugin.enabled && githubReleaseEndpoint(plugin)));
  let checkedOnline = $state(false);
  let checkingOnline = $state(false);
  let availableReleases = $state<Record<string, string>>({});

  $effect(() => {
    if (active && !currentPlugin && updateCheckerStore.automaticUpdateChecks && onlinePlugins.length && !checkedOnline) {
      untrack(() => { void checkPluginReleases(false); });
    }
  });

  async function checkPluginReleases(manual: boolean) {
    if (checkingOnline) return;
    checkedOnline = true;
    checkingOnline = true;
    try {
      const releases: Record<string, string> = {};
      let failures = false;
      for (const plugin of onlinePlugins) {
        try {
          const tag = await newerPluginRelease(plugin);
          if (tag) releases[plugin.id] = tag;
        } catch {
          failures = true;
        }
      }
      availableReleases = releases;
      if (manual && failures) snackbar.show(t("settings.updatesCheckFailed"), "warning");
      else if (manual && !Object.keys(releases).length) snackbar.show(t("settings.updatesUpToDate"), "success");
    } finally {
      checkingOnline = false;
    }
  }

  async function handleUpdate(plugin: InstalledPlugin) {
    contextMenuPlugin = null;
    contextMenuPos = null;
    const tag = availableReleases[plugin.id];
    await pluginsStore.update(plugin.id, tag);
    if (tag && pluginsStore.plugins.some(item => item.id === plugin.id && `v${item.version.replace(/^v/, "")}` === tag)) {
      delete availableReleases[plugin.id];
    }
  }

  function formatPermission(perm: string): string {
    if (perm === "network") return t("plugins.permNetwork");
    if (perm === "filesystem:downloads") return t("plugins.permDownloads");
    if (perm.startsWith("filesystem:")) return perm.replace("filesystem:", "Filesystem: ");
    return perm;
  }

  function getPermissionScopes(permissions?: any): string[] {
    if (!permissions) return [];
    if (Array.isArray(permissions)) {
      return permissions.filter(
        (p) =>
          typeof p === "string" &&
          !p.startsWith("sharedCredential:") &&
          !p.startsWith("shared_credential:") &&
          !p.startsWith("sharedCredentials:") &&
          !p.startsWith("shared:")
      );
    }
    if (permissions && Array.isArray(permissions.scopes)) {
      return permissions.scopes;
    }
    return [];
  }


  function formatDate(iso?: string | null): string {
    if (!iso) return "—";
    try {
      const d = new Date(iso);
      return isNaN(d.getTime())
        ? iso
        : d.toLocaleDateString(undefined, {
            year: "numeric",
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
          });
    } catch {
      return iso;
    }
  }
</script>

<div class="tab-layout flex flex-col h-full bg-gray-900 text-gray-100 select-none">
  {#if currentPlugin}
    <PluginRenderer pluginId={currentPlugin.id} screen={selectedPluginScreen} active={true}>
      {#snippet header(actions)}
        <!-- Embedded Plugin View with Top Bar -->
        <div class="px-6 py-3 border-b border-white/10 bg-gray-900 flex items-center justify-between shrink-0 gap-4">
          <div class="flex items-center gap-3 min-w-0">
            <button
              type="button"
              onclick={handleBackToPlugins}
              class="btn-secondary h-9 px-3 text-xs font-semibold inline-flex items-center gap-2 text-gray-300 hover:text-white cursor-pointer shrink-0"
              title={t("plugins.backToPlugins")}
              aria-label={t("plugins.backToPlugins")}
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
              </svg>
              <span>{t("plugins.backToPlugins")}</span>
            </button>

            <div class="h-5 w-px bg-white/10 shrink-0"></div>

            <div class="flex items-center gap-2.5 min-w-0">
              <div class="w-7 h-7 rounded-lg bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 flex items-center justify-center shrink-0 overflow-hidden">
                {#if getPluginImageSrc(currentPlugin.manifest, currentPlugin.iconUrl)}
                  <img src={getPluginImageSrc(currentPlugin.manifest, currentPlugin.iconUrl)} alt="" class="w-full h-full object-contain rounded-lg" />
                {:else if currentPlugin.manifest.tab?.icon === "film" || currentPlugin.manifest.icon === "film"}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
                {:else if currentPlugin.manifest.tab?.icon === "search" || currentPlugin.manifest.icon === "search"}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
                {/if}
              </div>
              <h2 class="text-sm font-bold text-white truncate">{currentPlugin.name}</h2>
              <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300 shrink-0">v{currentPlugin.version}</span>
            </div>
          </div>


          <div class="flex items-center gap-2 shrink-0">
            {@render actions()}
          </div>
        </div>
      {/snippet}
    </PluginRenderer>
  {:else}
    <!-- Content Band (Scrollable) -->
    <div class="flex-1 min-h-0 overflow-y-auto p-6 flex flex-col gap-6 scrollbar-thin">
      <!-- Install Card -->
      <div class="rounded-2xl border border-white/10 bg-white/[0.02] p-5 flex flex-col gap-4 shadow-sm">
        <div class="flex items-center justify-between gap-4 flex-wrap">
          <h2 class="text-sm font-semibold text-white flex items-center gap-2">
            <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
            </svg>
            {t("plugins.installFromRepo")}
          </h2>

          <!-- Segmented Toggle: Remote Repo vs Local Folder -->
          <div class="inline-flex items-center p-1 rounded-xl bg-black/40 border border-white/10 gap-1 text-xs select-none">
            <button
              type="button"
              onclick={() => installMode = "repo"}
              class="px-3 py-1.5 rounded-lg font-semibold transition-colors cursor-pointer {installMode === 'repo' ? 'bg-white/15 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
            >
              {t("plugins.modeRepo")}
            </button>
            <button
              type="button"
              onclick={() => installMode = "folder"}
              class="px-3 py-1.5 rounded-lg font-semibold transition-colors cursor-pointer {installMode === 'folder' ? 'bg-white/15 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
            >
              {t("plugins.modeFolder")}
            </button>
          </div>
        </div>

        {#if installMode === "repo"}
          <div class="h-10 min-h-[40px] flex flex-col sm:flex-row gap-3 items-center">
            <input
              type="text"
              placeholder="https://github.com/owner/repository"
              bind:value={urlInput}
              class="flex-1 w-full h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
            />
            <button
              type="button"
              disabled={!urlInput.trim() || inspecting}
              onclick={handleInspect}
              class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer shrink-0"
            >
              {#if inspecting}
                <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                {t("plugins.inspecting")}
              {:else}
                {t("plugins.inspect")}
              {/if}
            </button>
          </div>
        {:else}
          <div class="h-10 min-h-[40px] flex flex-col sm:flex-row gap-3 items-center">
            <div class="flex-1 w-full h-10">
              <PathPickerField
                value={urlInput}
                placeholder={t("plugins.localFolderPlaceholder")}
                browseTitle={t("plugins.browse")}
                browseLabel={t("flashcards.browse")}
                onbrowse={handleBrowseFolder}
                onclear={() => (urlInput = "")}
                containerClass="h-10"
                inputClass="h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
                browseButtonClass="btn-secondary h-10 px-3.5 rounded-xl text-xs font-semibold"
                clearButtonClass="h-10 px-3 inline-flex shrink-0 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 hover:border-red-400/60 hover:bg-red-500/20 transition-colors cursor-pointer"
              />
            </div>
            <button
              type="button"
              disabled={!urlInput.trim() || inspecting}
              onclick={handleInspect}
              class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer shrink-0"
            >
              {#if inspecting}
                <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                {t("plugins.inspecting")}
              {:else}
                {t("plugins.inspect")}
              {/if}
            </button>
          </div>
        {/if}

        <!-- Permission confirmation inspect banner/modal -->
        {#if inspectResult}
          {@const scopes = getPermissionScopes(inspectResult.manifest.permissions)}

          <div class="mt-2 p-5 rounded-xl border border-amber-500/30 bg-amber-500/10 flex flex-col gap-4 animate-in fade-in duration-150">
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3.5 min-w-0">
                <div class="w-12 h-12 rounded-xl bg-amber-500/20 border border-amber-500/30 text-amber-300 flex items-center justify-center shrink-0 overflow-hidden">
                  {#if getPluginImageSrc(inspectResult.manifest, inspectResult.iconUrl)}
                    <img src={getPluginImageSrc(inspectResult.manifest, inspectResult.iconUrl)} alt="" class="w-full h-full object-contain rounded-lg" />
                  {:else if inspectResult.manifest.tab?.icon === 'film' || inspectResult.manifest.icon === 'film'}
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
                  {:else if inspectResult.manifest.tab?.icon === 'search' || inspectResult.manifest.icon === 'search'}
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                  {:else}
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
                  {/if}
                </div>
                <div class="flex flex-col gap-1 min-w-0">
                  <div class="flex items-center gap-2.5 flex-wrap">
                    <span class="text-base font-bold text-white truncate">{inspectResult.manifest.name}</span>
                    <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{inspectResult.manifest.version}</span>
                    <span class="text-xs text-gray-400 font-mono">({inspectResult.manifest.id})</span>
                  </div>
                  {#if inspectResult.manifest.description}
                    <p class="text-xs text-gray-300 mt-1">{inspectResult.manifest.description}</p>
                  {/if}
                  {#if inspectResult.manifest.author}
                    <span class="text-xs text-gray-400">{t("plugins.author")}: {inspectResult.manifest.author}</span>
                  {/if}
                </div>
              </div>
              <button
                type="button"
                onclick={handleCancelInspect}
                aria-label={t("common.close")}
                class="text-gray-400 hover:text-white transition-colors p-1"
              >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
              </button>
            </div>

            <!-- Permissions list -->
            <div class="flex flex-col gap-1.5">
              <span class="text-xs font-semibold text-amber-300 uppercase tracking-wider">{t("plugins.requestedPermissions")}:</span>
              {#if scopes.length === 0}
                <span class="text-xs text-gray-400 italic">{t("plugins.noPermissions")}</span>
              {:else}
                <div class="flex flex-wrap gap-2 pt-1">
                  {#each scopes as perm}
                    <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/20 text-amber-200 border border-amber-400/30">
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
                      {formatPermission(perm)}
                    </span>
                  {/each}

                </div>
              {/if}
            </div>

            <!-- Untrusted code warning -->
            <div class="flex items-start gap-3 p-3 rounded-xl bg-black/30 border border-amber-500/20 text-xs text-amber-200/90 leading-relaxed">
              <svg class="w-5 h-5 text-amber-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>
              <div>
                <strong class="font-semibold">{t("plugins.untrustedCodeWarningTitle")}:</strong>
                {t("plugins.untrustedCodeWarning")}
              </div>
            </div>

            <div class="flex items-center justify-end gap-3 pt-1">
              <button
                type="button"
                class="btn-secondary text-xs px-4 py-2 cursor-pointer"
                onclick={handleCancelInspect}
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                disabled={installing}
                onclick={handleConfirmInstall}
                class="btn-primary text-xs px-5 py-2 font-semibold inline-flex items-center gap-2 cursor-pointer"
              >
                {#if installing}
                  <svg class="animate-spin h-3.5 w-3.5 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
                  {t("plugins.installing")}
                {:else}
                  {t("plugins.confirmInstall")}
                {/if}
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Installed Plugins Section -->
      <div class="flex flex-col gap-4">
        <!-- Section Header with Integrated Search Filter -->
        <div class="flex items-center justify-between gap-4 flex-wrap">
          <h2 class="text-sm font-semibold text-white">
            {t("plugins.installedList")} ({pluginsStore.plugins.length})
          </h2>

          {#if onlinePlugins.length}
            <button type="button" class="btn-secondary btn-compact" disabled={checkingOnline}
              onclick={() => checkPluginReleases(true)}>
              {t(checkingOnline ? "settings.updatesChecking" : "plugins.checkUpdates")}
            </button>
          {/if}
          <SearchInput
            bind:value={searchQuery}
            placeholder={t("plugins.searchPlaceholder")}
            class="w-64"
          />
        </div>

        {#if pluginsStore.loading && pluginsStore.plugins.length === 0}
          <div class="flex flex-col gap-3">
            <div class="skeleton w-full h-24 rounded-xl"></div>
            <div class="skeleton w-full h-24 rounded-xl"></div>
          </div>
        {:else if filteredPlugins.length === 0}
          <div class="empty-state flex flex-col items-center justify-center p-12 text-center rounded-2xl border border-white/5 bg-white/[0.01]">
            <div class="w-14 h-14 rounded-2xl bg-white/5 flex items-center justify-center text-gray-500 mb-3">
              <svg class="w-7 h-7" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
            </div>
            <h3 class="text-sm font-semibold text-gray-300">
              {searchQuery ? t("plugins.noSearchResults") : t("plugins.noInstalled")}
            </h3>
            <p class="text-xs text-gray-500 max-w-sm mt-1">
              {searchQuery ? t("plugins.clearSearchPrompt") : t("plugins.installPrompt")}
            </p>
          </div>
        {:else}
          <div class="grid grid-cols-1 gap-4">
            {#each filteredPlugins as plugin (plugin.id)}
              <div role="group" aria-label={plugin.name}
                class="p-5 rounded-2xl border border-white/10 bg-white/[0.03] transition-colors flex flex-col sm:flex-row sm:items-center justify-between gap-4 {plugin.enabled ? 'hover:border-white/20' : ''}"
                oncontextmenu={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  contextMenuPlugin = plugin;
                  contextMenuPos = { x: e.clientX, y: e.clientY };
                }}
              >
                <div class="relative flex items-start gap-4 min-w-0 flex-1 {plugin.enabled ? '' : 'grayscale opacity-65'}">
                  {#if plugin.enabled}
                    <button type="button"
                      class="absolute inset-0 rounded-xl cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-4"
                      aria-label={`${t("plugins.open")} · ${plugin.name}`}
                      onclick={() => handleOpenPlugin(plugin)}
                    ></button>
                  {/if}
                  <div class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 border overflow-hidden {plugin.enabled ? 'bg-indigo-500/10 border-indigo-500/30 text-indigo-400' : 'bg-white/5 border-white/10 text-gray-500'}">
                    {#if getPluginImageSrc(plugin.manifest, plugin.iconUrl)}
                      <img src={getPluginImageSrc(plugin.manifest, plugin.iconUrl)} alt="" class="w-full h-full object-contain rounded-lg" />
                    {:else if plugin.manifest.tab?.icon === 'film' || plugin.manifest.icon === 'film'}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
                    {:else if plugin.manifest.tab?.icon === 'search' || plugin.manifest.icon === 'search'}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
                    {:else}
                      <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
                    {/if}
                  </div>
                  <div class="flex flex-col min-w-0 flex-1">
                    <div class="flex items-center gap-2 flex-wrap">
                      <span class="text-base font-bold text-white truncate">{plugin.name}</span>
                      <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{plugin.version}</span>
                      {#if availableReleases[plugin.id]}
                        <span class="rounded bg-cyan-500/15 px-2 py-0.5 text-xs text-cyan-300">
                          {t("settings.updatesNewVersionAvailable", { version: availableReleases[plugin.id] })}
                        </span>
                      {/if}
                      {#if plugin.enabled}
                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{t("plugins.enabled")}</span>
                      {:else}
                        <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/10 text-gray-400 border border-white/5">{t("plugins.disabled")}</span>
                      {/if}
                    </div>
                    {#if plugin.description}
                      <p class="text-xs text-gray-400 mt-1 line-clamp-2">{plugin.description}</p>
                    {/if}
                    <div class="flex items-center gap-4 mt-2 text-xs text-gray-500 flex-wrap">
                      <span class="truncate">ID: <code class="text-gray-400 font-mono">{plugin.id}</code></span>
                      <span class="truncate">{t("plugins.installedSource")}: <span class="text-gray-400 font-mono truncate" title={plugin.sourceUrl}>{plugin.sourceUrl}</span></span>
                    </div>
                  </div>
                </div>

                <!-- Action buttons -->
                <div class="flex items-center gap-2.5 shrink-0 self-end sm:self-center">
                  {#if plugin.enabled}
                    <button
                      type="button"
                      class="btn-primary h-8 px-3 text-xs rounded-lg font-semibold inline-flex items-center gap-1.5 cursor-pointer"
                      onclick={() => handleOpenPlugin(plugin)}
                      title={t("plugins.open")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
                      </svg>
                      <span>{t("plugins.open")}</span>
                    </button>
                  {/if}

                  <!-- Enable / Disable Button: color-coded -->
                  {#if plugin.enabled}
                    <button
                      type="button"
                      class="text-amber-400 border border-amber-500/25 bg-amber-500/10 hover:bg-amber-500/20 inline-flex items-center gap-1.5 h-8 px-3 text-xs rounded-lg font-medium transition-colors cursor-pointer"
                      onclick={() => pluginsStore.disable(plugin.id)}
                      title={t("plugins.disable")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
                      <span>{t("plugins.disable")}</span>
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="text-gray-300 border border-white/15 bg-white/5 hover:bg-white/10 inline-flex items-center gap-1.5 h-8 px-3 text-xs rounded-lg font-medium transition-colors cursor-pointer"
                      onclick={() => pluginsStore.enable(plugin.id)}
                      title={t("plugins.enable")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                      <span>{t("plugins.enable")}</span>
                    </button>
                  {/if}

                  <!-- 2x2 Square Grid of 4 Icon Actions: Update, Details (i), Settings, Delete -->
                  <div class="grid grid-cols-2 gap-1.5 pl-1 border-l border-white/10">
                    <!-- Update (icon-only, emerald) -->
                    <button
                      type="button"
                      class="w-7 h-7 text-emerald-400 border border-emerald-500/25 bg-emerald-500/10 hover:bg-emerald-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!plugin.enabled}
                      onclick={() => handleUpdate(plugin)}
                      title={t("plugins.update")}
                      aria-label={t("plugins.update")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
                    </button>

                    <!-- Details (i) (icon-only, sky blue) -->
                    <button
                      type="button"
                      class="w-7 h-7 text-sky-400 border border-sky-500/25 bg-sky-500/10 hover:bg-sky-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!plugin.enabled}
                      onclick={() => openInfoModal(plugin)}
                      title={t("plugins.info")}
                      aria-label={t("plugins.info")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                    </button>

                    <!-- Settings (icon-only, slate) -->
                    <button
                      type="button"
                      class="w-7 h-7 text-gray-300 border border-white/15 bg-white/5 hover:bg-white/10 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!plugin.enabled}
                      onclick={() => openSettings(plugin)}
                      title={t("common.settings")}
                      aria-label={t("common.settings")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      </svg>
                    </button>

                    <!-- Delete (icon-only, rose/red) -->
                    <button
                      type="button"
                      class="w-7 h-7 text-rose-400 border border-rose-500/25 bg-rose-500/10 hover:bg-rose-500/20 inline-flex items-center justify-center rounded-lg transition-colors cursor-pointer"
                      onclick={() => openUninstallConfirm(plugin)}
                      title={t("plugins.uninstall")}
                      aria-label={t("plugins.uninstall")}
                    >
                      <svg class="w-3.5 h-3.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                      </svg>
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Fixed Bottom Band (Identical h-[92px] layout across Vesta) -->
  <div class="footer-band h-[92px] px-6 border-t border-white/10 bg-gray-900 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3 text-xs text-gray-400">
      {#if currentPlugin}
        <span class="font-semibold text-gray-200">{currentPlugin.name}</span>
        <span>•</span>
        <span class="font-mono text-gray-400">v{currentPlugin.version}</span>
        <span>•</span>
        <span class="text-emerald-400">{t("plugins.active")}</span>
      {:else}
        <span>{pluginsStore.plugins.length} {t("plugins.totalInstalled")}</span>
        <span>•</span>
        <span class="text-emerald-400">{pluginsStore.enabledPlugins.length} {t("plugins.active")}</span>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={() => (showSdkModal = true)}
        title={t("plugins.developerDocs")}
        class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors inline-flex items-center gap-1 cursor-pointer"
      >
        {t("plugins.developerDocs")} →
      </button>
    </div>
  </div>

  <!-- Built-in Developer Guide & SDK Modal -->
  {#if showSdkModal}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onclick={() => (showSdkModal = false)}
      onkeydown={(e) => { if (e.key === "Escape") showSdkModal = false; }}
    >
      <div
        class="w-full max-w-3xl max-h-[85vh] rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-4 shadow-2xl overflow-hidden"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between border-b border-white/10 pb-3 shrink-0">
          <div class="flex items-center gap-2.5">
            <h3 class="text-base font-bold text-white">{t("plugins.developerDocs")}</h3>
            <span class="text-xs font-mono px-2 py-0.5 rounded bg-indigo-500/20 text-indigo-300 border border-indigo-500/30">v1</span>
          </div>
          <div class="flex items-center gap-2">
            <button
              type="button"
              onclick={() => void openUrl("https://github.com/pierspad/vesta/blob/dev/docs/PLUGINS.md")}
              class="btn-secondary text-xs px-3 py-1.5 inline-flex items-center gap-1.5 cursor-pointer"
              title="docs/PLUGINS.md"
            >
              <span>GitHub</span>
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
            </button>
            <button
              type="button"
              class="text-gray-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/5 cursor-pointer"
              aria-label={t("common.close")}
              onclick={() => (showSdkModal = false)}
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
        <div class="flex-1 min-h-0 overflow-y-auto pr-2 text-xs scrollbar-thin">
          <MarkdownContent text={SDK_GUIDE_MARKDOWN} />
        </div>
        <div class="flex justify-end pt-3 border-t border-white/10 shrink-0">
          <button
            type="button"
            class="btn-secondary text-xs px-5 py-2 font-semibold cursor-pointer"
            onclick={() => (showSdkModal = false)}
          >
            {t("common.close")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Right-Click Context Menu -->
  {#if contextMenuPlugin && contextMenuPos}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50"
      onclick={() => { contextMenuPlugin = null; contextMenuPos = null; }}
      oncontextmenu={(e) => { e.preventDefault(); contextMenuPlugin = null; contextMenuPos = null; }}
    >
      <div
        class="absolute min-w-[180px] overflow-hidden rounded-xl border border-white/10 bg-gray-900/98 p-1.5 shadow-2xl z-50 text-xs backdrop-blur-md"
        style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
        onclick={(e) => e.stopPropagation()}
      >
        {#if contextMenuPlugin.enabled}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; handleOpenPlugin(p); }}
          >
            <svg class="w-4 h-4 text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
            {t("plugins.open")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={!contextMenuPlugin.enabled}
          onclick={() => { const p = contextMenuPlugin!; openInfoModal(p); }}
        >
          <svg class="w-4 h-4 text-sky-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
          {t("plugins.info")}
        </button>
        {#if contextMenuPlugin.manifest.tab?.settingsScreen || contextMenuPlugin.manifest.settings?.length}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={!contextMenuPlugin.enabled}
            onclick={() => { const p = contextMenuPlugin!; openSettings(p); }}
          >
            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/></svg>
            {t("common.settings")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer disabled:grayscale disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={!contextMenuPlugin.enabled}
          onclick={() => { const p = contextMenuPlugin!; handleUpdate(p); }}
        >
          <svg class="w-4 h-4 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/></svg>
          {t("plugins.update")}
        </button>
        <div class="my-1 border-t border-white/10"></div>
        {#if contextMenuPlugin.enabled}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; pluginsStore.disable(p.id); }}
          >
            <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"/></svg>
            {t("plugins.disable")}
          </button>
        {:else}
          <button
            type="button"
            class="w-full text-left px-3 py-2 rounded-lg text-gray-200 hover:bg-white/10 hover:text-white flex items-center gap-2 cursor-pointer"
            onclick={() => { const p = contextMenuPlugin!; contextMenuPlugin = null; contextMenuPos = null; pluginsStore.enable(p.id); }}
          >
            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
            {t("plugins.enable")}
          </button>
        {/if}
        <button
          type="button"
          class="w-full text-left px-3 py-2 rounded-lg text-rose-400 hover:bg-rose-500/10 hover:text-rose-300 flex items-center gap-2 cursor-pointer"
          onclick={() => { const p = contextMenuPlugin!; openUninstallConfirm(p); }}
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
          {t("common.delete")}
        </button>
      </div>
    </div>
  {/if}

  <!-- Extended Plugin Info Modal -->
  {#if showInfoModal && infoTarget}
    {@const scopes = getPermissionScopes(infoTarget.manifest.permissions)}

    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onclick={() => { showInfoModal = false; infoTarget = null; }}
      onkeydown={(e) => { if (e.key === 'Escape') { showInfoModal = false; infoTarget = null; } }}
    >
      <div
        class="w-full max-w-2xl max-h-[85vh] rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl overflow-hidden"
        onclick={(e) => e.stopPropagation()}
      >
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-white/10 pb-4 shrink-0">
          <div class="flex items-center gap-3 min-w-0">
            <div class="w-10 h-10 rounded-xl bg-indigo-500/10 border border-indigo-500/30 text-indigo-400 flex items-center justify-center shrink-0 overflow-hidden">
              {#if getPluginImageSrc(infoTarget.manifest, infoTarget.iconUrl)}
                <img src={getPluginImageSrc(infoTarget.manifest, infoTarget.iconUrl)} alt="" class="w-full h-full object-contain rounded-lg" />
              {:else if infoTarget.manifest.tab?.icon === 'film' || infoTarget.manifest.icon === 'film'}
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z"/></svg>
              {:else if infoTarget.manifest.tab?.icon === 'search' || infoTarget.manifest.icon === 'search'}
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
              {:else}
                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/></svg>
              {/if}
            </div>
            <div class="min-w-0">
              <div class="flex items-center gap-2 flex-wrap">
                <h3 class="text-base font-bold text-white truncate">{infoTarget.name} — {t("plugins.info")}</h3>
                <span class="text-xs font-mono px-2 py-0.5 rounded bg-white/10 text-gray-300">v{infoTarget.version}</span>
                {#if infoTarget.enabled}
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">{t("plugins.enabled")}</span>
                {:else}
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/10 text-gray-400 border border-white/5">{t("plugins.disabled")}</span>
                {/if}
              </div>
              <p class="text-xs text-gray-400 font-mono mt-0.5 truncate">ID: {infoTarget.id}</p>
            </div>
          </div>
          <button
            type="button"
            class="text-gray-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/5 cursor-pointer"
            aria-label={t("common.close")}
            onclick={() => { showInfoModal = false; infoTarget = null; }}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <!-- Content Scroll -->
        <div class="flex-1 min-h-0 overflow-y-auto pr-1 flex flex-col gap-5 scrollbar-thin">
          <!-- Metadata Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-4 rounded-xl border border-white/5 bg-white/[0.02] text-xs">
            {#if infoTarget.author}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.author")}</span>
                <span class="text-gray-200">{infoTarget.author}</span>
              </div>
            {/if}
            <div class="flex flex-col gap-0.5">
              <span class="text-gray-500 font-medium">{t("plugins.installedSource")}</span>
              <span class="text-gray-200 truncate font-mono" title={infoTarget.sourceUrl}>{infoTarget.sourceUrl}</span>
            </div>
            <div class="flex flex-col gap-0.5">
              <span class="text-gray-500 font-medium">{t("plugins.installedAt")}</span>
              <span class="text-gray-200">{formatDate(infoTarget.installedAt)}</span>
            </div>
            {#if infoTarget.updatedAt}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.updatedAt")}</span>
                <span class="text-gray-200">{formatDate(infoTarget.updatedAt)}</span>
              </div>
            {/if}
            {#if infoTarget.manifest.homepage}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.homepage")}</span>
                <a
                  href={infoTarget.manifest.homepage}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => { e.preventDefault(); void openUrl(infoTarget!.manifest.homepage!); }}
                  class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
                >
                  {infoTarget.manifest.homepage}
                  <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                </a>
              </div>
            {/if}
            {#if infoTarget.manifest.repository}
              <div class="flex flex-col gap-0.5">
                <span class="text-gray-500 font-medium">{t("plugins.repository")}</span>
                <a
                  href={infoTarget.manifest.repository}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => { e.preventDefault(); void openUrl(infoTarget!.manifest.repository!); }}
                  class="text-indigo-400 hover:text-indigo-300 hover:underline truncate inline-flex items-center gap-1 cursor-pointer"
                >
                  {infoTarget.manifest.repository}
                  <svg class="w-3 h-3 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                </a>
              </div>
            {/if}
          </div>

          <!-- Permissions Section -->
          <div class="flex flex-col gap-2">
            <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.requestedPermissions")}</span>
            {#if scopes.length === 0}
              <span class="text-xs text-gray-500 italic">{t("plugins.noPermissions")}</span>
            {:else}
              <div class="flex flex-wrap gap-2">
                {#each scopes as perm}
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium bg-amber-400/15 text-amber-300 border border-amber-400/25">
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/></svg>
                    {formatPermission(perm)}
                  </span>
                {/each}

              </div>
            {/if}
          </div>

          <!-- Short Description -->
          {#if infoTarget.description}
            <div class="flex flex-col gap-1.5">
              <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.description")}</span>
              <p class="text-sm text-gray-200 leading-relaxed">{infoTarget.description}</p>
            </div>
          {/if}

          <!-- README / Long Description -->
          {#if infoTarget.manifest.longDescription || infoTarget.readme || infoTarget.manifest.readme}
            {@const readmeText = infoTarget.manifest.longDescription || infoTarget.readme || infoTarget.manifest.readme || ""}
            <div class="flex flex-col gap-2 pt-2 border-t border-white/10">
              <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">{t("plugins.readme")}</span>
              <div class="p-4 rounded-xl border border-white/5 bg-black/30 overflow-x-auto text-xs">
                <MarkdownContent text={readmeText} />
              </div>
            </div>
          {/if}
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-3 pt-3 border-t border-white/10 shrink-0">
          <button
            type="button"
            class="btn-secondary text-xs px-5 py-2 font-semibold cursor-pointer"
            onclick={() => { showInfoModal = false; infoTarget = null; }}
          >
            {t("common.close")}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Uninstall confirmation dialog -->
  {#if uninstallTarget}
    <ConfirmDialog
      show={true}
      title={t("plugins.uninstallConfirmTitle")}
      message={`${t("plugins.uninstallConfirmMessage")} '${uninstallTarget.name}'?`}
      confirmText={t("common.delete")}
      cancelText={t("common.cancel")}
      variant="danger"
      on:confirm={confirmUninstall}
      on:cancel={() => uninstallTarget = null}
    />
  {/if}

  <!-- Plugin Settings Modal -->
  {#if settingsTarget && settingsTarget.manifest.settings}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
      <div class="w-full max-w-lg rounded-2xl border border-white/10 bg-gray-900 p-6 flex flex-col gap-5 shadow-2xl">
        <div class="flex items-center justify-between border-b border-white/10 pb-3">
          <h3 class="text-base font-bold text-white">{settingsTarget.name} — {t("common.settings")}</h3>
          <button
            type="button"
            class="text-gray-400 hover:text-white cursor-pointer p-1"
            aria-label={t("common.close")}
            onclick={() => settingsTarget = null}
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
          </button>
        </div>

        <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto scrollbar-thin">
          {#each settingsTarget.manifest.settings as field}
            {#if !field.expertOnly || uiMode.expertMode}
              {#if field.type === "checkbox"}
                <button
                  type="button"
                  role="checkbox"
                  aria-checked={Boolean(editSettingsValues[field.id])}
                  class="flex items-center gap-3 p-3 rounded-xl border transition-all text-left cursor-pointer {Boolean(editSettingsValues[field.id]) ? 'border-indigo-500/50 bg-indigo-500/10 text-white' : 'border-white/10 bg-white/[0.03] text-gray-300 hover:bg-white/[0.06]'}"
                  onclick={() => editSettingsValues[field.id] = !editSettingsValues[field.id]}
                >
                  <div class="w-5 h-5 rounded-lg border flex items-center justify-center transition-colors {Boolean(editSettingsValues[field.id]) ? 'border-indigo-400 bg-indigo-600 text-white' : 'border-white/20 bg-black/20 text-transparent'}">
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
                <div class="flex flex-col gap-1.5">
                  <label for="modal-setting-{field.id}" class="text-xs font-medium text-gray-400">{field.label}</label>
                  <input
                    id="modal-setting-{field.id}"
                    type={field.type === "password" ? "password" : field.type === "number" ? "number" : "text"}
                    bind:value={editSettingsValues[field.id]}
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
                          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/></svg>
                          {info.link.label}
                        </a>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/if}
            {/if}
          {/each}
        </div>

        <div class="flex justify-end gap-3 pt-3 border-t border-white/10">
          <button
            type="button"
            class="btn-secondary text-xs px-4 py-2 cursor-pointer"
            onclick={() => settingsTarget = null}
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            class="btn-primary text-xs px-5 py-2 font-semibold cursor-pointer"
            onclick={savePluginSettings}
          >
            {t("plugins.save")}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<script lang="ts">
  import { isInputActive } from "$lib/utils/keyboardFocus";
  import { preloadTabs } from "$lib/utils/preloadTabs";
  import { locale } from "$lib/i18n";
  let t = $derived($locale);
  import ExtractTabComponent from "$lib/tabs/ExtractTab.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, type Component } from "svelte";
  import { manageWindowSize } from "$lib/utils/windowLifecycle";
  import { createTabLoader } from "$lib/utils/tabLoader";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import { exportFormatStore } from "$lib/stores/exportFormatStore.svelte";
  import { whisperModelsStore } from "$lib/stores/whisperModelsStore.svelte";
  import ShortcutOverlay from "$lib/components/ShortcutOverlay.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import AppContextMenu from "$lib/components/AppContextMenu.svelte";
  import AppTooltip from "$lib/components/AppTooltip.svelte";
  import Snackbar from "$lib/components/Snackbar.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { getShortcuts } from "$lib/utils/shortcuts";
  import * as vestaConfig from "$lib/config/vestaConfig";

  type AppTab = "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "refine" | "extract" | "experimental";

  type LazyComponent = Component<any>;
  const tabLoaders: Record<AppTab, () => Promise<{ default: LazyComponent }>> = {
    flashcards: () => import("$lib/tabs/FlashcardsTab.svelte"),
    refine: () => import("$lib/tabs/RefineTab.svelte"),
    translate: () => import("$lib/tabs/TranslateTab.svelte"),
    sync: () => import("$lib/tabs/SyncTab.svelte"),
    align: () => import("$lib/tabs/AlignTab.svelte"),
    transcribe: () => import("$lib/tabs/TranscribeTab.svelte"),
    extract: () => Promise.resolve({ default: ExtractTabComponent }),
    experimental: () => import("$lib/tabs/ExperimentalTab.svelte"),
    settings: () => import("$lib/tabs/SettingsTab.svelte"),
  };
  // The lightweight extraction shell is ready on the first tab switch.
  let loadedTabs = $state<Partial<Record<AppTab, LazyComponent>>>({ extract: ExtractTabComponent });
  let tabLoadErrors = $state<Partial<Record<AppTab, string>>>({});
  let firstRunSetupModal = $state<LazyComponent | null>(null);

  const ensureTabLoaded = createTabLoader<AppTab, LazyComponent>({
    loaders: tabLoaders,
    getLoaded: () => loadedTabs,
    setLoaded: (tab, component) => { loadedTabs[tab] = component; },
    clearError: (tab) => { delete tabLoadErrors[tab]; },
    setError: (tab, message) => { tabLoadErrors[tab] = message; },
    onError: (tab, error) => console.error(`Failed to load the ${tab} tab`, error),
  });

  const openWhisperAfterSetup = vestaConfig.getItem("vesta-setup-open-whisper") === "true";
  if (openWhisperAfterSetup) vestaConfig.removeItem("vesta-setup-open-whisper");
  const setupDestination = vestaConfig.getItem("vesta-setup-next-tab");
  const initialTab = setupDestination === "transcribe" || setupDestination === "extract" || setupDestination === "flashcards" ? setupDestination : openWhisperAfterSetup ? "settings" : "flashcards";
  if (setupDestination) vestaConfig.removeItem("vesta-setup-next-tab");
  let activeTab = $state<AppTab>(initialTab);
  let showFirstRunSetup = $state(
    vestaConfig.getItem("vesta-first-run-force") === "true" ||
      (vestaConfig.isNewInstallation() && vestaConfig.getItem("vesta-first-run-setup-complete") !== "true"),
  );

  $effect(() => {
    void ensureTabLoaded(activeTab);
  });

  $effect(() => {
    if (showFirstRunSetup && !firstRunSetupModal) {
      void import("$lib/modals/FirstRunSetupModal.svelte").then(({ default: component }) => {
        firstRunSetupModal = component;
      });
    }
  });

  async function completeFirstRunSetup(_wantsTranscription: boolean, destination: "flashcards" | "transcribe" | "extract") {
    vestaConfig.removeItem("vesta-setup-open-whisper");
    vestaConfig.setItem("vesta-setup-next-tab", destination);
    await vestaConfig.flush();
    showFirstRunSetup = false;
    ankiStore.reinit();
    exportFormatStore.reinit();
    whisperModelsStore.reinit();
    changeTab(destination);
  }
  const initialPreference = (() => {
    const savedPref = vestaConfig.getItem("vesta-sidebar-user-pref");
    if (savedPref === "collapsed" || savedPref === "expanded") {
      return savedPref;
    }
    // Migration from old key
    const oldSaved = vestaConfig.getItem("vesta-sidebar-collapsed");
    if (oldSaved === "true") return "collapsed";
    if (oldSaved === "false") return "expanded";
    return null;
  })();

  let userPreference = $state<"collapsed" | "expanded" | null>(initialPreference);
  let sidebarCollapsed = $state(initialPreference === "collapsed");
  let requestedSettingsSection = $state<"overview" | "llm" | "whisper" | "language" | "anki" | "diagnostics" | "shortcuts">(openWhisperAfterSetup ? "whisper" : "overview");
  let highlightItemId = $state<string | null>(null);
  let lastActiveMainTab = $state<Exclude<AppTab, "settings">>(initialTab === "settings" ? "flashcards" : initialTab);

  let shortcutsList = $state(getShortcuts());

  function reloadShortcuts() {
    shortcutsList = getShortcuts();
  }


  function matchShortcut(e: KeyboardEvent, shortcutKey: string): boolean {
    const parts = shortcutKey.split("+").map((p) => p.trim());
    
    const hasCtrl = parts.includes("Ctrl");
    const hasAlt = parts.includes("Alt");
    const hasShift = parts.includes("Shift");
    
    if (e.ctrlKey !== hasCtrl && e.metaKey !== hasCtrl) return false;
    if (e.altKey !== hasAlt) return false;
    if (e.shiftKey !== hasShift) return false;
    
    const mainKeyPart = parts.find((p) => !["Ctrl", "Alt", "Shift"].includes(p));
    if (!mainKeyPart) return false;
    
    let eventKey = e.key;
    if (eventKey === " ") eventKey = "Space";
    else if (eventKey.length === 1) eventKey = eventKey.toUpperCase();
    
    return eventKey === mainKeyPart;
  }

  function triggerGlobalAction(action: string) {
    if (action === "switchToFlashcards") {
      changeTab("flashcards");
    } else if (action === "switchToRefine") {
      changeTab("refine");
    } else if (action === "switchToTranslate") {
      if (!aiStore.killSwitchActive) changeTab("translate");
    } else if (action === "switchToSync") {
      changeTab("sync");
    } else if (action === "switchToAlign") {
      changeTab("align");
    } else if (action === "switchToTranscribe") {
      if (!aiStore.killSwitchActive) changeTab("transcribe");
    } else if (action === "switchToSettings") {
      goToSettings("overview");
    } else if (action === "switchToShortcuts") {
      goToSettings("shortcuts");
    } else if (action === "addApiKey") {
      goToSettings("llm");
      setTimeout(() => {
        window.dispatchEvent(new CustomEvent("vesta-open-add-key-modal"));
      }, 50);
    }
  }

  $effect(() => {
    if (aiStore.killSwitchActive) {
      if (activeTab === "translate" || activeTab === "transcribe") {
        changeTab("flashcards");
      }
      if (requestedSettingsSection === "llm" || requestedSettingsSection === "whisper") {
        requestedSettingsSection = "overview";
      }
    }
  });

  $effect(() => {
    if (activeTab === "flashcards" || activeTab === "settings" || activeTab === "experimental") {
      void ankiStore.checkConnection(true);
      const handleFocus = () => {
        void ankiStore.checkConnection(true);
      };
      window.addEventListener("focus", handleFocus);
      const interval = setInterval(() => {
        void ankiStore.checkConnection(true);
      }, 30000);
      return () => {
        clearInterval(interval);
        window.removeEventListener("focus", handleFocus);
      };
    }
  });

  const MIN_WIDTH = 460;
  const MIN_HEIGHT = 520;
  
  // Collapse threshold (approx. when the window gets close to the 1280px/1300px mark, 
  // before dropping to single-column layout).
  const COLLAPSE_THRESHOLD = 1300;
  // Re-expand threshold (COLLAPSE_THRESHOLD + 190px delta to avoid flicker)
  const EXPAND_THRESHOLD = COLLAPSE_THRESHOLD + 190;

  function applyResponsiveSidebar(logicalWidth: number) {
    // If the user explicitly set a preference, respect it and skip responsive auto-collapsing.
    if (userPreference !== null) {
      return;
    }

    if (logicalWidth <= COLLAPSE_THRESHOLD) {
      sidebarCollapsed = true;
    } else if (logicalWidth >= EXPAND_THRESHOLD) {
      sidebarCollapsed = false;
    }
  }

  const TABS_ORDER: AppTab[] = [
    "flashcards",
    "refine",
    "translate",
    "sync",
    "align",
    "transcribe",
    "extract",
    "experimental",
    "settings",
  ];
  function getMainTabForDigit(digit: number, isKillSwitchOn: boolean): AppTab | null {
    if (isKillSwitchOn) {
      switch (digit) {
        case 1: return "flashcards";
        case 2: return "refine";
        case 3: return "sync";
        case 4: return "align";
        default: return null;
      }
    } else {
      switch (digit) {
        case 1: return "flashcards";
        case 2: return "refine";
        case 3: return "translate";
        case 4: return "sync";
        case 5: return "align";
        case 6: return "transcribe";
        default: return null;
      }
    }
  }

  function getSettingsSectionForDigit(digit: number, isKillSwitchOn: boolean): "overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts" | null {
    if (isKillSwitchOn) {
      switch (digit) {
        case 1: return "overview";
        case 2: return "language";
        case 3: return "anki";
        case 4: return "shortcuts";
        default: return null;
      }
    } else {
      switch (digit) {
        case 1: return "overview";
        case 2: return "llm";
        case 3: return "whisper";
        case 4: return "language";
        case 5: return "anki";
        case 6: return "shortcuts";
        default: return null;
      }
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing) return;
    const inputActive = isInputActive();

    // Handle context-sensitive Alt+1..6 and Alt+Shift+1..6 navigation
    if (!inputActive && e.altKey && !e.ctrlKey && !e.metaKey) {
      let num: number | null = null;
      if (e.key >= "1" && e.key <= "6") {
        num = parseInt(e.key);
      } else {
        const matchDigit = e.code.match(/^Digit([1-6])$/);
        if (matchDigit) {
          num = parseInt(matchDigit[1]);
        }
      }

      if (num !== null) {
        const inSettings = activeTab === "settings";
        const killSwitch = aiStore.killSwitchActive;
        const hasShift = e.shiftKey;

        if (hasShift) {
          if (inSettings) {
            const targetTab = getMainTabForDigit(num, killSwitch);
            if (targetTab) {
              e.preventDefault();
              changeTab(targetTab);
              return;
            }
          } else {
            const targetSection = getSettingsSectionForDigit(num, killSwitch);
            if (targetSection) {
              e.preventDefault();
              goToSettings(targetSection);
              return;
            }
          }
        } else {
          if (inSettings) {
            const targetSection = getSettingsSectionForDigit(num, killSwitch);
            if (targetSection) {
              e.preventDefault();
              goToSettings(targetSection);
              return;
            }
          } else {
            const targetTab = getMainTabForDigit(num, killSwitch);
            if (targetTab) {
              e.preventDefault();
              changeTab(targetTab);
              return;
            }
          }
        }
      }
    }

    if (e.ctrlKey && (e.key === "PageDown" || e.key === "PageUp")) {
      if (!inputActive) {
        e.preventDefault();
        const currentIndex = TABS_ORDER.indexOf(activeTab);
        if (currentIndex !== -1) {
          let nextIndex;
          if (e.key === "PageDown") {
            nextIndex = (currentIndex + 1) % TABS_ORDER.length;
          } else {
            nextIndex = (currentIndex - 1 + TABS_ORDER.length) % TABS_ORDER.length;
          }
          changeTab(TABS_ORDER[nextIndex]);
        }
        return;
      }
    }

    if (!inputActive) {
      for (const shortcut of shortcutsList) {
        if (shortcut.category === "global" && matchShortcut(e, shortcut.defaultKey)) {
          e.preventDefault();
          triggerGlobalAction(shortcut.action);
          return;
        }
      }
    }
  }

  // Enforce minimum window size at runtime (Linux WMs may ignore config)
  onMount(() => {
    const appWindow = getCurrentWindow();
    const stopPreloading = preloadTabs<AppTab>(["settings", "translate", "sync", "transcribe", "refine", "align", "experimental"], ensureTabLoaded, (callback) => {
      if ("requestIdleCallback" in window) {
        const id = window.requestIdleCallback(callback, { timeout: 200 });
        return () => window.cancelIdleCallback(id);
      }
      const id = setTimeout(callback, 0);
      return () => clearTimeout(id);
    });
    const stopManagingWindowSize = manageWindowSize(appWindow, { width: MIN_WIDTH, height: MIN_HEIGHT }, (logicalWidth) => {
      applyResponsiveSidebar(logicalWidth);
    });

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("shortcuts-updated", reloadShortcuts);

    const handleWindowResize = () => {
      if (window.innerWidth < MIN_WIDTH) return;
      applyResponsiveSidebar(window.innerWidth);
    };
    window.addEventListener("resize", handleWindowResize);

    applyResponsiveSidebar(window.innerWidth);

    return () => {
      stopPreloading();
      stopManagingWindowSize();
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("shortcuts-updated", reloadShortcuts);
      window.removeEventListener("resize", handleWindowResize);
    };
  });

  // Expose function to change tab programmatically
  function changeTab(tab: AppTab) {
    if (tab === "settings") {
      goToSettings(requestedSettingsSection);
      return;
    }
    activeTab = tab;
    lastActiveMainTab = tab;
  }

  function goToSettings(section: typeof requestedSettingsSection = "overview", highlightId?: string) {
    requestedSettingsSection = section;
    highlightItemId = highlightId || null;
    if (activeTab !== "settings") {
      lastActiveMainTab = activeTab;
    }
    activeTab = "settings";
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    userPreference = sidebarCollapsed ? "collapsed" : "expanded";
    vestaConfig.setItem("vesta-sidebar-user-pref", userPreference);
    // Keep old key synced for backwards compatibility
    vestaConfig.setItem("vesta-sidebar-collapsed", String(sidebarCollapsed));
  }

  // Make available globally for TranslateTab link
  if (typeof window !== 'undefined') {
    (window as any).changeTab = changeTab;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<main
  class="flex h-screen min-w-[460px] min-h-[520px] bg-gray-900 text-gray-100"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'; }}
  ondrop={(e) => e.preventDefault()}
>
  <Sidebar {activeTab} onTabChange={changeTab} onTabPreload={(tab) => void ensureTabLoaded(tab)} collapsed={sidebarCollapsed} onToggleCollapse={toggleSidebar} bind:settingsSection={requestedSettingsSection} {lastActiveMainTab} />

  <!-- Main Content - use CSS visibility to preserve state -->
  <div class="flex-1 overflow-hidden relative">
    {#if loadedTabs.translate}
      {@const TranslateTab = loadedTabs.translate}
      <div class="absolute inset-0" class:hidden={activeTab !== "translate"}><TranslateTab onGoToSettings={goToSettings} active={activeTab === "translate"} /></div>
    {/if}
    {#if loadedTabs.sync}
      {@const SyncTab = loadedTabs.sync}
      <div class="absolute inset-0" class:hidden={activeTab !== "sync"}><SyncTab active={activeTab === "sync"} /></div>
    {/if}
    {#if loadedTabs.transcribe}
      {@const TranscribeTab = loadedTabs.transcribe}
      <div class="absolute inset-0" class:hidden={activeTab !== "transcribe"}><TranscribeTab onGoToSettings={goToSettings} active={activeTab === "transcribe"} /></div>
    {/if}
    {#if loadedTabs.align}
      {@const AlignTab = loadedTabs.align}
      <div class="absolute inset-0" class:hidden={activeTab !== "align"}><AlignTab active={activeTab === "align"} /></div>
    {/if}
    {#if loadedTabs.flashcards}
      {@const FlashcardsTab = loadedTabs.flashcards}
      <div class="absolute inset-0" class:hidden={activeTab !== "flashcards"}><FlashcardsTab onGoToSettings={goToSettings} active={activeTab === "flashcards"} /></div>
    {/if}
    {#if loadedTabs.refine}
      {@const RefineTab = loadedTabs.refine}
      <div class="absolute inset-0" class:hidden={activeTab !== "refine"}><RefineTab onGoToSettings={goToSettings} active={activeTab === "refine"} /></div>
    {/if}
    {#if loadedTabs.extract}
      {@const ExtractTab = loadedTabs.extract}
      <div class="absolute inset-0" class:hidden={activeTab !== "extract"}><ExtractTab active={activeTab === "extract"} /></div>
    {/if}
    {#if loadedTabs.experimental}
      {@const ExperimentalTab = loadedTabs.experimental}
      <div class="absolute inset-0" class:hidden={activeTab !== "experimental"}><ExperimentalTab /></div>
    {/if}
    {#if loadedTabs.settings}
      {@const SettingsTab = loadedTabs.settings}
      <div class="absolute inset-0" class:hidden={activeTab !== "settings"}><SettingsTab active={activeTab === "settings"} bind:requestedSection={requestedSettingsSection} bind:highlightItemId={highlightItemId} /></div>
    {/if}
    {#if !loadedTabs[activeTab]}
      <div class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-sm text-gray-500">
        {#if tabLoadErrors[activeTab]}
          <p>{t("common.loadFailed")}</p>
          <button class="btn-secondary px-4 py-2" onclick={() => void ensureTabLoaded(activeTab)}>{t("common.retry")}</button>
        {:else}
          <p>{t("common.loading")}</p>
        {/if}
      </div>
    {/if}
  </div>

  {#if snackbar.message}
    {@const snackbarKey = snackbar.key}
    <Snackbar
      message={snackbar.message}
      variant={snackbar.variant}
      duration={snackbar.duration}
      animationKey={snackbar.key}
      paused={snackbar.paused}
      onpause={(reason) => snackbar.pause(reason)}
      onresume={(reason) => snackbar.resume(reason)}
      onclose={() => snackbar.close(snackbarKey)}
    />
  {/if}

  <ShortcutOverlay {activeTab} />
  <AppContextMenu />
  {#if showFirstRunSetup && firstRunSetupModal}
    {@const FirstRunSetupModal = firstRunSetupModal}
    <FirstRunSetupModal onComplete={completeFirstRunSetup} />
  {/if}
</main>

<AppTooltip />

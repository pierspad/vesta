<script lang="ts">
  import { PhysicalSize } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, type Component } from "svelte";
  import { ankiStore } from "$lib/stores/ankiStore.svelte";
  import ShortcutOverlay from "$lib/components/ShortcutOverlay.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import AppContextMenu from "$lib/components/AppContextMenu.svelte";
  import Snackbar from "$lib/components/Snackbar.svelte";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { aiStore } from "$lib/stores/aiStore.svelte";
  import { getShortcuts } from "$lib/utils/shortcuts";
  import * as vestaConfig from "$lib/config/vestaConfig";

  type AppTab = "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "refine" | "experimental";

  type LazyComponent = Component<any>;
  const tabLoaders: Record<AppTab, () => Promise<{ default: LazyComponent }>> = {
    flashcards: () => import("$lib/tabs/FlashcardsTab.svelte"),
    refine: () => import("$lib/tabs/RefineTab.svelte"),
    translate: () => import("$lib/tabs/TranslateTab.svelte"),
    sync: () => import("$lib/tabs/SyncTab.svelte"),
    align: () => import("$lib/tabs/AlignTab.svelte"),
    transcribe: () => import("$lib/tabs/TranscribeTab.svelte"),
    experimental: () => import("$lib/tabs/ExperimentalTab.svelte"),
    settings: () => import("$lib/tabs/SettingsTab.svelte"),
  };
  const pendingTabLoads = new Map<AppTab, Promise<void>>();
  let loadedTabs = $state<Partial<Record<AppTab, LazyComponent>>>({});
  let tabLoadErrors = $state<Partial<Record<AppTab, string>>>({});
  let firstRunSetupModal = $state<LazyComponent | null>(null);

  function ensureTabLoaded(tab: AppTab): Promise<void> {
    if (loadedTabs[tab]) return Promise.resolve();
    const pending = pendingTabLoads.get(tab);
    if (pending) return pending;
    delete tabLoadErrors[tab];
    const load = tabLoaders[tab]()
      .then(({ default: component }) => {
        loadedTabs[tab] = component;
      })
      .catch((error) => {
        console.error(`Failed to load the ${tab} tab`, error);
        tabLoadErrors[tab] = error instanceof Error ? error.message : String(error);
      })
      .finally(() => {
        pendingTabLoads.delete(tab);
      });
    pendingTabLoads.set(tab, load);
    return load;
  }

  let activeTab = $state<AppTab>("flashcards");
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

  function completeFirstRunSetup(wantsTranscription: boolean) {
    showFirstRunSetup = false;
    if (wantsTranscription) goToSettings("whisper");
    // Stores are constructed at startup; reload once so every selected default
    // is applied consistently without a half-old, half-new session.
    window.location.reload();
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
  let requestedSettingsSection = $state<"overview" | "llm" | "whisper" | "language" | "anki" | "diagnostics" | "shortcuts">("overview");
  let highlightItemId = $state<string | null>(null);
  let lastActiveMainTab = $state<Exclude<AppTab, "settings">>("flashcards");

  let shortcutsList = $state(getShortcuts());

  function reloadShortcuts() {
    shortcutsList = getShortcuts();
  }

  function isInputActive(): boolean {
    const active = document.activeElement;
    if (!active) return false;
    const tag = active.tagName.toLowerCase();
    return tag === "input" || tag === "textarea" || active.getAttribute("contenteditable") === "true";
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
      ankiStore.checkConnection();
      const interval = setInterval(() => {
        ankiStore.checkConnection();
      }, 5000);
      return () => {
        clearInterval(interval);
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
    let unlisten: (() => void) | null = null;

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("shortcuts-updated", reloadShortcuts);

    const handleWindowResize = () => {
      if (window.innerWidth < MIN_WIDTH) return;
      applyResponsiveSidebar(window.innerWidth);
    };
    window.addEventListener("resize", handleWindowResize);

    (async () => {
      const scaleFactor = await appWindow.scaleFactor();
      const physMinW = Math.round(MIN_WIDTH * scaleFactor);
      const physMinH = Math.round(MIN_HEIGHT * scaleFactor);

      applyResponsiveSidebar(window.innerWidth);

      await appWindow.setMinSize(new PhysicalSize(physMinW, physMinH)).catch(() => {});

      // Fallback: enforce min size on resize events for WMs that ignore setMinSize
      unlisten = await appWindow.onResized(async ({ payload: size }) => {
        if (size.width === 0 || size.height === 0) return;
        if (size.width < physMinW || size.height < physMinH) {
          const w = Math.max(size.width, physMinW);
          const h = Math.max(size.height, physMinH);
          await appWindow.setSize(new PhysicalSize(w, h)).catch(() => {});
        }
        const logicalWidth = size.width / scaleFactor;
        if (logicalWidth >= MIN_WIDTH) {
          applyResponsiveSidebar(logicalWidth);
        }
      });
    })();

    return () => {
      unlisten?.();
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
  <Sidebar {activeTab} onTabChange={changeTab} collapsed={sidebarCollapsed} onToggleCollapse={toggleSidebar} bind:settingsSection={requestedSettingsSection} {lastActiveMainTab} />

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
          <p>Unable to load this section.</p>
          <button class="btn-secondary px-4 py-2" onclick={() => void ensureTabLoaded(activeTab)}>Retry</button>
        {:else}
          <p>Loading…</p>
        {/if}
      </div>
    {/if}
  </div>

  {#if snackbar.message}
    <Snackbar
      message={snackbar.message}
      variant={snackbar.variant}
      duration={snackbar.duration}
      animationKey={snackbar.key}
      onclose={() => snackbar.close()}
    />
  {/if}

  <ShortcutOverlay {activeTab} />
  <AppContextMenu />
  {#if showFirstRunSetup && firstRunSetupModal}
    {@const FirstRunSetupModal = firstRunSetupModal}
    <FirstRunSetupModal onComplete={completeFirstRunSetup} />
  {/if}
</main>

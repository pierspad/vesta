<script lang="ts">
  import { PhysicalSize } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import FlashcardsTab from "./lib/FlashcardsTab.svelte";
  import SettingsTab from "./lib/SettingsTab.svelte";
  import ShortcutOverlay from "./lib/ShortcutOverlay.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import SyncTab from "./lib/SyncTab.svelte";
  import TranscribeTab from "./lib/TranscribeTab.svelte";
  import TranslateTab from "./lib/TranslateTab.svelte";
  import AlignTab from "./lib/AlignTab.svelte";
  import AppContextMenu from "./lib/AppContextMenu.svelte";
  import Snackbar from "./lib/Snackbar.svelte";
  import { snackbar } from "./lib/snackbarStore.svelte";

  type AppTab = "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings";

  let activeTab = $state<AppTab>("flashcards");
  let sidebarCollapsed = $state(
    typeof localStorage !== 'undefined'
      ? localStorage.getItem("vesta-sidebar-collapsed") === "true"
      : false
  );
  let requestedSettingsSection = $state<"overview" | "llm" | "whisper" | "language" | "anki" | "shortcuts">("overview");
  let highlightItemId = $state<string | null>(null);

  const MIN_WIDTH = 460;
  const MIN_HEIGHT = 520;
  // Hysteresis: collapse early while shrinking, expand only when there's plenty
  // of space while growing. This keeps columns as the first thing to recover.
  const SIDEBAR_COLLAPSE_FIRST_WIDTH = 1560;
  // Re-expand only after content has had time to fully recover width first.
  // Delta is slightly larger than the sidebar width gain (w-20 -> w-[238px] = 158px)
  // to preserve hysteresis and avoid flicker around the switching point.
  const SIDEBAR_EXPAND_LAST_WIDTH = SIDEBAR_COLLAPSE_FIRST_WIDTH + 190;

  function applyResponsiveSidebar(logicalWidth: number) {
    const savedState = typeof localStorage !== 'undefined' ? localStorage.getItem("vesta-sidebar-collapsed") : null;
    
    // If the user explicitly collapsed it (savedState === "true"), it should stay collapsed.
    // Otherwise, we only auto-collapse it if the screen gets critically narrow (<= 1024px).
    const collapseThreshold = (savedState === "true") ? 0 : 1024;
    const expandThreshold = (savedState === "true") ? 0 : 1024 + 190;

    if (logicalWidth <= collapseThreshold) {
      sidebarCollapsed = true;
      return;
    }
    if (logicalWidth >= expandThreshold) {
      sidebarCollapsed = false;
    }
  }

  const TABS_ORDER: AppTab[] = [
    "flashcards",
    "translate",
    "sync",
    "align",
    "transcribe",
    "settings",
  ];

  function handleKeyDown(e: KeyboardEvent) {
    if (e.ctrlKey && (e.key === "PageDown" || e.key === "PageUp")) {
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
    }
  }

  // Enforce minimum window size at runtime (Linux WMs may ignore config)
  onMount(() => {
    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | null = null;

    window.addEventListener("keydown", handleKeyDown);

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
      window.removeEventListener("resize", handleWindowResize);
    };
  });

  // Expose function to change tab programmatically
  function changeTab(tab: AppTab) {
    if (tab === "settings") {
      goToSettings("overview");
      return;
    }
    activeTab = tab;
  }

  function goToSettings(section: typeof requestedSettingsSection = "overview", highlightId?: string) {
    requestedSettingsSection = section;
    highlightItemId = highlightId || null;
    activeTab = "settings";
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem("vesta-sidebar-collapsed", String(sidebarCollapsed));
    }
  }

  // Make available globally for TranslateTab link
  if (typeof window !== 'undefined') {
    (window as any).changeTab = changeTab;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<main
  class="flex h-screen min-w-[460px] min-h-[520px] bg-gradient-to-br from-gray-900 via-gray-950 to-gray-900 text-gray-100"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'; }}
  ondrop={(e) => e.preventDefault()}
>
  <Sidebar {activeTab} onTabChange={changeTab} collapsed={sidebarCollapsed} onToggleCollapse={toggleSidebar} bind:settingsSection={requestedSettingsSection} />

  <!-- Main Content - use CSS visibility to preserve state -->
  <div class="flex-1 overflow-hidden relative">
    <div class="absolute inset-0" class:hidden={activeTab !== "translate"}>
      <TranslateTab onGoToSettings={goToSettings} active={activeTab === "translate"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "sync"}>
      <SyncTab active={activeTab === "sync"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "transcribe"}>
      <TranscribeTab onGoToSettings={goToSettings} active={activeTab === "transcribe"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "align"}>
      <AlignTab active={activeTab === "align"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "flashcards"}>
      <FlashcardsTab active={activeTab === "flashcards"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "settings"}>
      <SettingsTab bind:requestedSection={requestedSettingsSection} bind:highlightItemId={highlightItemId} />
    </div>
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
</main>

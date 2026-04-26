<script lang="ts">
  import { PhysicalSize } from "@tauri-apps/api/dpi";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import FlashcardsTab from "./lib/FlashcardsTab.svelte";
  import NotificationsTab from "./lib/NotificationsTab.svelte";
  import SettingsTab from "./lib/SettingsTab.svelte";
  import ShortcutOverlay from "./lib/ShortcutOverlay.svelte";
  import ShortcutsTab from "./lib/ShortcutsTab.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import SyncTab from "./lib/SyncTab.svelte";
  import TranscribeTab from "./lib/TranscribeTab.svelte";
  import TranslateTab from "./lib/TranslateTab.svelte";
  import AlignTab from "./lib/AlignTab.svelte";

  type AppTab = "translate" | "sync" | "transcribe" | "align" | "flashcards" | "settings" | "notifications" | "shortcuts";

  let activeTab = $state<AppTab>("flashcards");
  let sidebarCollapsed = $state(false);
  let requestedSettingsSection = $state<"overview" | "llm" | "whisper" | "language" | "anki">("overview");

  const MIN_WIDTH = 760;
  const MIN_HEIGHT = 620;
  // Hysteresis: collapse early while shrinking, expand only when there's plenty
  // of space while growing. This keeps columns as the first thing to recover.
  const SIDEBAR_COLLAPSE_FIRST_WIDTH = 1560;
  // Re-expand only after content has had time to fully recover width first.
  // Delta is slightly larger than the sidebar width gain (w-20 -> w-72 = 208px)
  // to preserve hysteresis and avoid flicker around the switching point.
  const SIDEBAR_EXPAND_LAST_WIDTH = SIDEBAR_COLLAPSE_FIRST_WIDTH + 240;

  function applyResponsiveSidebar(logicalWidth: number) {
    if (logicalWidth <= SIDEBAR_COLLAPSE_FIRST_WIDTH) {
      sidebarCollapsed = true;
      return;
    }
    if (logicalWidth >= SIDEBAR_EXPAND_LAST_WIDTH) {
      sidebarCollapsed = false;
    }
  }

  // Enforce minimum window size at runtime (Linux WMs may ignore config)
  onMount(() => {
    const appWindow = getCurrentWindow();
    let unlisten: (() => void) | null = null;

    (async () => {
      const scaleFactor = await appWindow.scaleFactor();
      const physMinW = Math.round(MIN_WIDTH * scaleFactor);
      const physMinH = Math.round(MIN_HEIGHT * scaleFactor);

      applyResponsiveSidebar(window.innerWidth);

      await appWindow.setMinSize(new PhysicalSize(physMinW, physMinH)).catch(() => {});

      // Fallback: enforce min size on resize events for WMs that ignore setMinSize
      unlisten = await appWindow.onResized(async ({ payload: size }) => {
        if (size.width < physMinW || size.height < physMinH) {
          const w = Math.max(size.width, physMinW);
          const h = Math.max(size.height, physMinH);
          await appWindow.setSize(new PhysicalSize(w, h)).catch(() => {});
        }
        applyResponsiveSidebar(size.width / scaleFactor);
      });
    })();

    return () => { unlisten?.(); };
  });

  // Expose function to change tab programmatically
  function changeTab(tab: AppTab) {
    if (tab === "settings") {
      goToSettings("overview");
      return;
    }
    activeTab = tab;
  }

  function goToSettings(section: typeof requestedSettingsSection = "overview") {
    requestedSettingsSection = section;
    activeTab = "settings";
    window.dispatchEvent(new CustomEvent("vesta-open-settings-section", { detail: section }));
  }

  function handleTabChange(tab: AppTab) {
    if (tab === "settings") {
      goToSettings("overview");
      return;
    }
    activeTab = tab;
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
  }

  // Make available globally for TranslateTab link
  if (typeof window !== 'undefined') {
    (window as any).changeTab = changeTab;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<main
  class="flex h-screen min-w-[760px] min-h-[620px] bg-gradient-to-br from-gray-900 via-gray-950 to-gray-900 text-gray-100"
  ondragover={(e) => { e.preventDefault(); if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'; }}
  ondrop={(e) => e.preventDefault()}
>
  <Sidebar {activeTab} onTabChange={handleTabChange} collapsed={sidebarCollapsed} onToggleCollapse={toggleSidebar} />

  <!-- Main Content - use CSS visibility to preserve state -->
  <div class="flex-1 overflow-hidden relative">
    <div class="absolute inset-0" class:hidden={activeTab !== "translate"}>
      <TranslateTab onGoToSettings={goToSettings} active={activeTab === "translate"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "sync"}>
      <SyncTab active={activeTab === "sync"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "transcribe"}>
      <TranscribeTab onGoToSettings={goToSettings} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "align"}>
      <AlignTab />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "flashcards"}>
      <FlashcardsTab active={activeTab === "flashcards"} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "settings"}>
      <SettingsTab requestedSection={requestedSettingsSection} />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "notifications"}>
      <NotificationsTab />
    </div>
    <div class="absolute inset-0" class:hidden={activeTab !== "shortcuts"}>
      <ShortcutsTab />
    </div>
  </div>

  <ShortcutOverlay {activeTab} />
</main>

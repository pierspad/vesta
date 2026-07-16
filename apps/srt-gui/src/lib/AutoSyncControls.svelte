<script lang="ts">
  import { locale } from "./i18n";
  import { aiStore } from "./aiStore.svelte";
  import { autoSyncStore } from "./autoSyncStore.svelte";

  interface Props {
    canAutoSync: boolean;
    onStart: () => void;
  }

  let { canAutoSync, onStart }: Props = $props();

  let t = $derived($locale);
</script>

{#if !aiStore.killSwitchActive}
  <!-- Unified Autosync Button Group -->
  <div class="flex items-center bg-white/5 border border-white/10 rounded-xl p-1 shrink-0 relative group">
    <!-- Autosync Trigger Button -->
    <button
      onclick={onStart}
      disabled={autoSyncStore.isAutoSyncing || !canAutoSync}
      class="px-5 py-2 bg-indigo-600/80 hover:bg-indigo-500/80 border border-indigo-500/30 disabled:bg-indigo-600/40 text-indigo-100 rounded-lg font-bold text-sm transition-all shadow-lg shadow-indigo-950/20 flex items-center gap-2 enabled:hover:scale-[1.02] enabled:active:scale-[0.98] disabled:opacity-55 shrink-0 {!canAutoSync ? 'pointer-events-none saturate-75' : 'cursor-pointer'}"
    >
      {#if autoSyncStore.isAutoSyncing}
        <svg class="animate-spin w-4 h-4 text-indigo-300" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
      {:else}
        <svg
          class="w-4 h-4 text-indigo-100"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
          />
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      {/if}
      <span>{t("sync.autoSync") || "Autosync"}</span>
    </button>

    <!-- Selector Capsule Switcher -->
    <div class="flex items-center bg-black/25 border border-white/5 rounded-lg p-0.5 ml-2 shrink-0 select-none relative">
      <!-- Sliding indicator background -->
      <div
        class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-indigo-500/20 border border-indigo-500/50 shadow shadow-indigo-500/25 transition-all duration-200 ease-out"
        style="width: 100px; transform: translateX({autoSyncStore.selectedMode === 'quick' ? '0px' : '100px'});"
      ></div>

      <button
        onclick={() => autoSyncStore.toggleMode()}
        disabled={autoSyncStore.isAutoSyncing || !canAutoSync}
        class="w-[100px] py-1 rounded-md text-[10px] font-bold transition-colors duration-200 flex items-center justify-center gap-1.5 cursor-pointer disabled:cursor-not-allowed select-none relative z-10
          {autoSyncStore.selectedMode === 'quick' ? 'text-indigo-200' : 'text-gray-500 hover:text-gray-300'}
          {!canAutoSync ? 'opacity-30 pointer-events-none' : ''}"
      >
        <svg class="w-3.5 h-3.5 flex-shrink-0 transition-colors duration-200 {autoSyncStore.selectedMode === 'quick' ? 'text-indigo-300' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
        <span>{autoSyncStore.formatModeName("sync.autoSyncFast", "Breve")}</span>
      </button>
      <button
        onclick={() => autoSyncStore.toggleMode()}
        disabled={autoSyncStore.isAutoSyncing || !canAutoSync}
        class="w-[100px] py-1 rounded-md text-[10px] font-bold transition-colors duration-200 flex items-center justify-center gap-1.5 cursor-pointer disabled:cursor-not-allowed select-none relative z-10
          {autoSyncStore.selectedMode === 'precise' ? 'text-indigo-200' : 'text-gray-500 hover:text-gray-300'}
          {!canAutoSync ? 'opacity-30 pointer-events-none' : ''}"
      >
        <svg class="w-3.5 h-3.5 flex-shrink-0 transition-colors duration-200 {autoSyncStore.selectedMode === 'precise' ? 'text-indigo-300' : 'text-gray-500'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
        </svg>
        <span>{autoSyncStore.formatModeName("sync.autoSyncFull", "Preciso")}</span>
      </button>

      <!-- Custom Tooltip for Autosync Group -->
      {#if !canAutoSync}
        <div
          class="pointer-events-none absolute bottom-full z-50 mb-3 -translate-x-1/2 rounded-xl border border-indigo-400/30 bg-gray-950/95 p-3 text-left text-xs text-indigo-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap"
          style="left: {autoSyncStore.selectedMode === 'quick' ? '52px' : '152px'};"
        >
          {t("sync.autoSyncRequires")}
        </div>
      {:else}
        <div
          class="pointer-events-none absolute bottom-full z-50 mb-3 -translate-x-1/2 rounded-xl border border-indigo-400/30 bg-gray-950/95 p-3 text-center text-xs text-indigo-300 shadow-2xl shadow-black/40 ring-1 ring-white/10 transition-all duration-150 delay-0 group-hover:delay-300 opacity-0 group-hover:opacity-100 group-hover:translate-y-0 translate-y-1 whitespace-nowrap"
          style="left: {autoSyncStore.selectedMode === 'quick' ? '52px' : '152px'};"
        >
          {autoSyncStore.selectedMode === "quick"
            ? `${t("sync.autoSync")} - ${autoSyncStore.formatModeName("sync.autoSyncFast", "Breve")}`
            : `${t("sync.autoSync")} - ${autoSyncStore.formatModeName("sync.autoSyncFull", "Preciso")}`}
        </div>
      {/if}
    </div>
  </div>
{/if}

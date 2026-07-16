<script lang="ts">
  import { locale } from "./i18n";
  import { autoSyncStore } from "./autoSyncStore.svelte";

  interface Props {
    onCancel: () => void;
  }

  let { onCancel }: Props = $props();

  let t = $derived($locale);
</script>

{#if autoSyncStore.isAutoSyncing}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-[100] bg-black/80 flex items-center justify-center p-6 backdrop-blur-sm"
       onclick={(e) => e.stopPropagation()}
       onkeydown={(e) => e.stopPropagation()}
  >
    <div class="max-w-md w-full p-8 text-center flex flex-col items-center bg-[#0f172a] border border-indigo-300/20 rounded-2xl shadow-2xl opacity-100">
      <svg class="animate-spin w-12 h-12 text-indigo-400 mb-6" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
      <h3 class="text-xl font-bold text-white mb-2">{t("sync.autoSyncInProgress")}</h3>
      <p class="text-indigo-300 text-sm mb-6 max-w-[280px] leading-relaxed">{autoSyncStore.message}</p>

      <div class="w-full bg-gray-800 rounded-full h-3 mb-2 overflow-hidden border border-white/5">
        <div
          class="bg-indigo-500 h-full rounded-full transition-all duration-300 ease-out relative overflow-hidden"
          style="width: {autoSyncStore.progress}%"
        >
          <div class="absolute inset-0 bg-white/20 animate-pulse"></div>
        </div>
      </div>
      <p class="text-gray-400 text-xs font-mono">{Math.round(autoSyncStore.progress)}%</p>
      <button
        onclick={onCancel}
        disabled={autoSyncStore.isCancelling}
        class="mt-6 px-4 py-2 border border-red-500/50 text-red-400 hover:bg-red-500/20 rounded-lg text-sm transition-colors disabled:opacity-50"
      >
        {autoSyncStore.isCancelling ? t("sync.autoSyncCancelling") : t("sync.autoSyncCancel")}
      </button>
    </div>
  </div>
{/if}

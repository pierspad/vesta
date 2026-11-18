<script lang="ts">
  import { locale } from "$lib/i18n";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import MarkdownContent from "$lib/components/MarkdownContent.svelte";
  import { SDK_GUIDE_MARKDOWN } from "./pluginHelpers";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();
  let t = $derived($locale);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
  onclick={onClose}
  onkeydown={(e) => { if (e.key === "Escape") onClose(); }}
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
          onclick={() => void openUrl("https://github.com/pierspad/vesta/blob/main/docs/PLUGINS.md")}
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
          onclick={onClose}
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
        onclick={onClose}
      >
        {t("common.close")}
      </button>
    </div>
  </div>
</div>

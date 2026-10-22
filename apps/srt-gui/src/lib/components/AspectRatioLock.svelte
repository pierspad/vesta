<script lang="ts">
  import { locale } from "$lib/i18n";
  let { ratio = $bindable<number | null>(null), width, height }: { ratio?: number | null; width: number; height: number } = $props();
  let t = $derived($locale);
</script>
<div class="flex h-11 items-center justify-center">
  <button type="button" aria-label={t("flashcards.keepAspectRatio")} title={t("flashcards.keepAspectRatio")} aria-pressed={ratio !== null}
    disabled={!(width > 0 && height > 0)}
    onclick={() => ratio = ratio === null ? width / height : null}
    class="flex h-8 w-8 items-center justify-center rounded-lg border transition-colors disabled:opacity-40 {ratio !== null ? 'bg-indigo-500/20 border-indigo-400/50 text-indigo-300' : 'border-white/10 text-gray-400 hover:bg-white/5'}">
    <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5" y="10" width="14" height="11" rx="2" />
      <path d={ratio !== null ? "M8 10V6a4 4 0 018 0v4" : "M8 10V6a4 4 0 018 0"} /><path d="M12 14v3" />
    </svg>
  </button>
</div>

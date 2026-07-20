<script lang="ts">
  import { locale } from "$lib/i18n";

  interface Props {
    total: number;
    messageKey: string;
    playButtonGradient?: boolean;
  }

  let { total, messageKey, playButtonGradient = false }: Props = $props();

  let t = $derived($locale);
</script>

<div class="w-full max-w-6xl flex flex-col gap-4 opacity-50 select-none pointer-events-none">
  <div class="text-center flex-shrink-0">
    <span
      class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-white/5 text-gray-500 text-sm font-medium border border-white/5"
    >
      <svg
        class="w-4 h-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
        /></svg
      >
      {t("sync.wizard.checkpoint")} — #0 / {total}
    </span>
  </div>

  <div
    class="bg-white/5 rounded-xl p-5 text-center flex-shrink-0 flex flex-col items-center justify-center min-h-[100px] border border-dashed border-white/10"
  >
    <p class="text-xl text-gray-400 font-medium leading-relaxed italic">
      {t(messageKey)}
    </p>
    <p class="text-sm text-gray-500 mt-3 font-mono">
      00:00.000 → 00:00.000
    </p>
  </div>

  <div class="flex-shrink-0">
    <div class="flex items-center gap-4">
      <span class="text-sm text-gray-500 font-mono w-24">00:00.000</span>
      <input
        type="range"
        min="0"
        max="100"
        value="0"
        disabled
        class="flex-1 opacity-30 cursor-not-allowed"
      />
      <span class="text-sm text-gray-500 font-mono w-24 text-right">00:00.000</span>
    </div>
  </div>

  <div
    class="flex items-center justify-center gap-4 flex-wrap flex-shrink-0 min-h-[60px]"
  >
    <button
      disabled
      class="w-14 h-14 flex items-center justify-center rounded-full {playButtonGradient
        ? 'bg-gradient-to-r from-gray-700 to-gray-800'
        : 'bg-gray-800'} text-gray-400 shadow-md transition-all cursor-not-allowed opacity-50"
      title={t("sync.playPause")}
    >
      <svg
        class="w-7 h-7 text-gray-400 ml-1"
        fill="currentColor"
        viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg
      >
    </button>

    <button
      disabled
      class="w-12 h-12 flex items-center justify-center rounded-full bg-white/5 hover:bg-white/10 transition-colors cursor-not-allowed opacity-50"
      title={t("sync.wizard.replay")}
    >
      <svg
        class="w-6 h-6 text-gray-400"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
        /></svg
      >
    </button>

    <div class="flex items-center gap-2 bg-white/5 rounded-xl px-4 py-2 opacity-50">
      <button
        disabled
        class="w-12 h-8 flex items-center justify-center bg-white/15 rounded-lg text-[11px] font-medium transition-colors cursor-not-allowed"
      >
        −3s
      </button>
      <button
        disabled
        class="w-12 h-8 flex items-center justify-center bg-white/15 rounded-lg text-[11px] font-medium transition-colors cursor-not-allowed"
      >
        −0.5s
      </button>
      <button
        disabled
        class="w-8 h-8 flex items-center justify-center bg-white/15 rounded-lg text-lg font-medium transition-colors cursor-not-allowed"
      >
        −
      </button>
      <div class="flex flex-col items-center min-w-[80px]">
        <span class="text-xs text-gray-500 uppercase tracking-wide"
          >{t("sync.offset")}</span
        >
        <span class="text-base font-mono font-medium text-white">+0.00s</span>
      </div>
      <button
        disabled
        class="w-8 h-8 flex items-center justify-center bg-white/15 rounded-lg text-lg font-medium transition-colors cursor-not-allowed"
      >
        +
      </button>
      <button
        disabled
        class="w-12 h-8 flex items-center justify-center bg-white/15 rounded-lg text-[11px] font-medium transition-colors cursor-not-allowed"
      >
        +0.5s
      </button>
      <button
        disabled
        class="w-12 h-8 flex items-center justify-center bg-white/15 rounded-lg text-[11px] font-medium transition-colors cursor-not-allowed"
      >
        +3s
      </button>
    </div>

    <button
      disabled
      class="btn-secondary py-3 px-6 flex items-center gap-2 text-base font-medium opacity-50 cursor-not-allowed"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" /></svg>
      {t("sync.wizard.skip")}
    </button>

    <button
      disabled
      class="btn-success py-3 px-6 flex items-center gap-2 opacity-50 text-base font-medium cursor-not-allowed"
    >
      <svg
        class="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M5 13l4 4L19 7"
        /></svg
      >
      {t("sync.wizard.confirm")}
    </button>
  </div>

  <div
    class="flex flex-wrap gap-3 text-xs text-gray-500 justify-center flex-shrink-0 opacity-50"
  >
    <div class="flex items-center gap-1">
      <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
        >{t("keys.space") || "Space"}</kbd
      ><span>{t("sync.playPause")}</span>
    </div>
    <div class="flex items-center gap-1">
      <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
        >←/→</kbd
      ><span>{t("sync.seek")}</span>
    </div>
    <div class="flex items-center gap-1">
      <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
        >↑/↓</kbd
      ><span>{t("sync.offsetAdjust")}</span>
    </div>
    <div class="flex items-center gap-1">
      <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
        >R</kbd
      ><span>{t("sync.wizard.replay")}</span>
    </div>
    <div class="flex items-center gap-1">
      <kbd class="px-2 py-1 bg-white/10 rounded text-gray-400"
        >{t("keys.enter") || "Enter"}</kbd
      ><span>{t("sync.confirm")}</span>
    </div>
  </div>
</div>

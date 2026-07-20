<script lang="ts">
  import { locale } from "./i18n";

  interface SyncStatus {
    is_loaded: boolean;
    srt_path: string | null;
    video_path: string | null;
    total_subtitles: number;
    anchor_count: number;
    checked_count: number;
    completion_percentage: number;
    average_offset_ms: number;
    suggested_next_id: number | null;
  }

  interface Props {
    status: SyncStatus | null;
    confidenceScore: number;
    formatOffset: (ms: number) => string;
  }

  let { status, confidenceScore, formatOffset }: Props = $props();

  let t = $derived($locale);
</script>

<div class="glass-card p-5 min-h-0 space-y-4 flex flex-col justify-between">
  <div>
    <h3 class="text-lg font-semibold flex items-center gap-2 mb-4 text-cyan-400">
      <svg
        class="w-5 h-5"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        ><path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
        /></svg
      >
      {t("sync.statusTitle")}
    </h3>

    <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
      <div class="bg-white/5 rounded-xl py-2 px-1 text-center flex flex-col justify-center min-w-0">
        <p class="text-xl font-bold text-white truncate">
          {status?.is_loaded ? status.total_subtitles : 0}
        </p>
        <p class="text-[10px] text-gray-500 uppercase tracking-wider mt-1 truncate">{t("sync.subtitles")}</p>
      </div>
      <div class="bg-white/5 rounded-xl py-2 px-1 text-center flex flex-col justify-center min-w-0">
        <p class="text-xl font-bold text-green-400 truncate">
          {status?.is_loaded ? status.anchor_count : 0}
        </p>
        <p class="text-[10px] text-gray-500 uppercase tracking-wider mt-1 truncate">{t("sync.anchors")}</p>
      </div>
      <div class="bg-white/5 rounded-xl py-2 px-1 text-center flex flex-col justify-center min-w-0">
        <p
          class="text-xl font-bold truncate {status?.is_loaded && status.average_offset_ms > 0
            ? 'text-green-400'
            : status?.is_loaded && status.average_offset_ms < 0
              ? 'text-red-400'
              : 'text-white'}"
        >
          {status?.is_loaded ? formatOffset(status.average_offset_ms) : '+0.00s'}
        </p>
        <p class="text-[10px] text-gray-500 uppercase tracking-wider mt-1 truncate">{t("sync.averageOffset")}</p>
      </div>
      <div class="bg-white/5 rounded-xl py-2 px-1 text-center flex flex-col justify-center min-w-0" title={t("sync.confidenceHelp") || "Confidence based on anchors"}>
        <p class="text-xl font-bold text-indigo-300 truncate">
          {status?.is_loaded ? confidenceScore : 0}%
        </p>
        <p class="text-[10px] text-gray-500 uppercase tracking-wider mt-1 truncate">{t("sync.confidence") || "Confidence"}</p>
      </div>
    </div>
  </div>

  <div class="space-y-1.5 pt-1.5 border-t border-white/5 flex-shrink-0">
    <div class="flex justify-between text-[10px] text-gray-400 font-semibold">
      <span class="uppercase tracking-wider">{t("sync.completed") || "Completato"}</span>
      <span>{status?.is_loaded ? status.completion_percentage.toFixed(1) : "0.0"}%</span>
    </div>
    <div class="progress-modern h-1.5 w-full bg-white/5 rounded-full overflow-hidden">
      <div
        class="progress-modern-bar bg-indigo-500 h-full rounded-full transition-all duration-300"
        style="width: {status?.is_loaded ? status.completion_percentage : 0}%"
      ></div>
    </div>
  </div>
</div>

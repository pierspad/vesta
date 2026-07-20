<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "./utils/dialogGuard";
  import { snackbar } from "./snackbarStore.svelte";
  import { locale } from "./i18n";
  import { getFileName } from "./models";
  import Card from "./components/Card.svelte";
  import SectionHeader from "./components/SectionHeader.svelte";
  import * as vestaConfig from "./vestaConfig";

  let t = $derived($locale);

  // ─── Condensed audio ────────────────────────────────────────────────────────

  let mediaPath = $state("");
  let srtPath = $state("");
  let outputPath = $state("");
  let detectMode = $state<"subtitles" | "vad">("subtitles");
  let padMs = $state(150);
  let mergeGapMs = $state(1500);
  let bitrate = $state(128);

  let condensing = $state(false);
  let condenseStage = $state("");
  let condenseCurrent = $state(0);
  let condenseTotal = $state(0);

  interface CondenseResult {
    success: boolean;
    message: string;
    outputPath: string;
    spans: number;
    outputDurationMs: number;
    inputDurationMs: number;
  }
  let condenseResult = $state<CondenseResult | null>(null);

  const canStartCondense = $derived(
    !!mediaPath && !!outputPath && (detectMode === "vad" || !!srtPath) && !condensing,
  );

  function fmtDuration(ms: number): string {
    const total = Math.round(ms / 1000);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    return h > 0 ? `${h}h ${m}m ${s}s` : `${m}m ${s}s`;
  }

  async function pickMedia() {
    const selected = await guardedOpen({
      filters: [
        { name: "Media", extensions: ["mkv", "mp4", "avi", "webm", "mp3", "m4a", "flac", "wav", "ogg", "opus", "aac"] },
      ],
    });
    if (selected && typeof selected === "string") {
      mediaPath = selected;
      if (!outputPath) {
        outputPath = selected.replace(/\.[^/.]+$/, "") + ".condensed.mp3";
      }
    }
  }

  async function pickSrt() {
    const selected = await guardedOpen({
      filters: [{ name: "SubRip (.srt)", extensions: ["srt"] }],
    });
    if (selected && typeof selected === "string") srtPath = selected;
  }

  async function pickOutput() {
    const selected = await guardedSave({
      defaultPath: outputPath || undefined,
      filters: [{ name: "MP3", extensions: ["mp3"] }],
    });
    if (selected && typeof selected === "string") outputPath = selected;
  }

  async function startCondense() {
    if (!canStartCondense) return;
    condensing = true;
    condenseResult = null;
    condenseStage = "detect";
    condenseCurrent = 0;
    condenseTotal = 0;

    const unlisten = await listen<{ stage: string; message: string; current: number; total: number }>(
      "condense-progress",
      (event) => {
        condenseStage = event.payload.stage;
        condenseCurrent = event.payload.current;
        condenseTotal = event.payload.total;
      },
    );

    try {
      const config = {
        media_path: mediaPath,
        output_path: outputPath,
        mode: detectMode === "vad" ? { type: "vad" } : { type: "subtitles", srt_path: srtPath },
        pad_ms: padMs,
        merge_gap_ms: mergeGapMs,
        bitrate_kbps: bitrate,
      };
      condenseResult = await invoke<CondenseResult>("condense_start", { config });
      snackbar.show(t("experimental.condense.done"), "success");
    } catch (err: any) {
      const message = err?.toString() ?? "";
      snackbar.show(
        message.includes("ERR_ALREADY_RUNNING") ? t("common.error.alreadyRunning") : message,
        "error",
      );
    } finally {
      unlisten();
      condensing = false;
    }
  }

  async function cancelCondense() {
    try {
      await invoke("condense_cancel");
    } catch {
      /* run già terminato */
    }
  }
</script>

<div class="h-full flex flex-col bg-gray-900 text-gray-100 overflow-hidden">
  <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-5 scrollbar-thin">
    <!-- Condensed audio -->
    <Card>
      <SectionHeader
        title={t("experimental.condense.title")}
        accent="amber"
        iconPath="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2z"
      />

      <!-- Files -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("experimental.condense.mediaFile")} <span class="text-red-400">*</span></span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(mediaPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={mediaPath || undefined} />
            <button onclick={pickMedia} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("experimental.condense.outputFile")} <span class="text-red-400">*</span></span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(outputPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={outputPath || undefined} />
            <button onclick={pickOutput} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
      </div>

      <!-- Detection mode -->
      <div>
        <span class="block text-xs text-gray-500 mb-2">{t("experimental.condense.detection")}</span>
        <div class="flex items-center gap-1 bg-white/5 border border-white/5 p-1 rounded-xl w-fit">
          <button
            onclick={() => (detectMode = "subtitles")}
            disabled={condensing}
            class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer {detectMode === 'subtitles' ? 'bg-amber-600 text-white shadow-md' : 'text-gray-400 hover:text-white hover:bg-white/5'}"
          >
            {t("experimental.condense.modeSubtitles")}
          </button>
          <button
            onclick={() => (detectMode = "vad")}
            disabled={condensing}
            class="px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer {detectMode === 'vad' ? 'bg-amber-600 text-white shadow-md' : 'text-gray-400 hover:text-white hover:bg-white/5'}"
          >
            {t("experimental.condense.modeVad")}
          </button>
        </div>
        <p class="text-[11px] text-gray-500 mt-1.5">
          {detectMode === "vad" ? t("experimental.condense.vadHint") : t("experimental.condense.subtitlesHint")}
        </p>
      </div>

      {#if detectMode === "subtitles"}
        <div class="md:w-1/2">
          <span class="block text-xs text-gray-500 mb-1">{t("experimental.condense.srtFile")} <span class="text-red-400">*</span></span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(srtPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={srtPath || undefined} />
            <button onclick={pickSrt} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
      {/if}

      <!-- Options -->
      <div class="grid grid-cols-3 gap-3 md:w-2/3">
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("experimental.condense.padding")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={padMs} min="0" step="50" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("experimental.condense.mergeGap")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={mergeGapMs} min="0" step="100" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="block text-xs text-gray-500 mb-1">{t("flashcards.audioBitrate")}</span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={bitrate} min="32" step="32" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">kb/s</span>
          </div>
        </div>
      </div>

      <!-- Actions + progress -->
      <div class="flex items-center gap-3">
        {#if condensing}
          <button onclick={cancelCondense} class="rounded-xl bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-sm font-bold text-red-100 px-5 py-2.5 shadow-md transition-all cursor-pointer">
            {t("common.cancel")}
          </button>
          <div class="flex-1 flex items-center gap-3">
            <div class="flex-1 bg-white/10 h-2 rounded-full overflow-hidden">
              <div
                class="bg-gradient-to-r from-amber-500 to-orange-500 h-full rounded-full transition-all duration-300"
                style="width: {condenseTotal > 0 ? (condenseCurrent / condenseTotal) * 100 : 5}%"
              ></div>
            </div>
            <span class="text-xs text-gray-400 whitespace-nowrap">
              {condenseStage === "detect" ? t("experimental.condense.stageDetect") : condenseTotal > 0 ? `${condenseCurrent}/${condenseTotal}` : "…"}
            </span>
          </div>
        {:else}
          <button
            onclick={startCondense}
            disabled={!canStartCondense}
            class="rounded-xl bg-amber-600/90 hover:bg-amber-500/90 border border-amber-500/30 disabled:opacity-40 disabled:cursor-not-allowed text-sm font-bold text-amber-50 px-5 py-2.5 shadow-md transition-all cursor-pointer flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            {t("experimental.condense.start")}
          </button>
        {/if}
      </div>

      {#if condenseResult}
        <div class="bg-emerald-500/5 border border-emerald-500/20 rounded-xl p-4 text-xs text-gray-300 flex flex-wrap gap-x-6 gap-y-1">
          <span><span class="font-bold text-emerald-300">{condenseResult.spans}</span> {t("experimental.condense.resultSegments")}</span>
          <span>{t("experimental.condense.resultDuration")}: <span class="font-bold text-emerald-300">{fmtDuration(condenseResult.outputDurationMs)}</span></span>
          {#if condenseResult.inputDurationMs > 0}
            <span>{t("experimental.condense.resultRatio")}: <span class="font-bold text-emerald-300">{Math.round((condenseResult.outputDurationMs / condenseResult.inputDurationMs) * 100)}%</span></span>
          {/if}
          <span class="text-gray-500 truncate w-full" title={condenseResult.outputPath}>{condenseResult.outputPath}</span>
        </div>
      {/if}
    </Card>
  </div>
</div>

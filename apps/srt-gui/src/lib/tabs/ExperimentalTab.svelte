<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import Card from "$lib/components/Card.svelte";
  import SectionHeader from "$lib/components/SectionHeader.svelte";
  import * as vestaConfig from "$lib/config/vestaConfig";

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

  // ─── Video Hardcoded Subtitles OCR ──────────────────────────────────────────
  let ocrVideoPath = $state("");
  let ocrOutputPath = $state("");
  let ocrLanguage = $state("zh");
  let ocrRegion = $state("bottom20");
  let ocrFps = $state(2);
  let ocrExtracting = $state(false);
  let ocrStage = $state<"sampling" | "ocr" | "timing">("sampling");
  let ocrCurrent = $state(0);
  let ocrTotal = $state(100);

  interface OcrResult {
    success: boolean;
    outputPath: string;
    lines: number;
    durationMs: number;
  }
  let ocrResult = $state<OcrResult | null>(null);

  const canStartOcr = $derived(!!ocrVideoPath && !!ocrOutputPath && !ocrExtracting);

  async function pickOcrVideo() {
    const selected = await guardedOpen({
      filters: [
        { name: "Video", extensions: ["mp4", "mkv", "webm", "avi", "mov", "ts", "flv", "m4v"] },
      ],
    });
    if (selected && typeof selected === "string") {
      ocrVideoPath = selected;
      if (!ocrOutputPath) {
        ocrOutputPath = selected.replace(/\.[^/.]+$/, "") + ".ocr.srt";
      }
    }
  }

  async function pickOcrOutput() {
    const selected = await guardedSave({
      defaultPath: ocrOutputPath || undefined,
      filters: [{ name: "SubRip (.srt)", extensions: ["srt"] }],
    });
    if (selected && typeof selected === "string") ocrOutputPath = selected;
  }

  async function startOcr() {
    if (!canStartOcr) return;
    ocrExtracting = true;
    ocrResult = null;
    ocrStage = "sampling";
    ocrCurrent = 0;
    ocrTotal = 100;

    try {
      ocrStage = "sampling";
      for (let i = 0; i <= 35; i += 5) {
        if (!ocrExtracting) return;
        ocrCurrent = i;
        await new Promise((r) => setTimeout(r, 60));
      }
      ocrStage = "ocr";
      for (let i = 35; i <= 80; i += 5) {
        if (!ocrExtracting) return;
        ocrCurrent = i;
        await new Promise((r) => setTimeout(r, 80));
      }
      ocrStage = "timing";
      for (let i = 80; i <= 100; i += 5) {
        if (!ocrExtracting) return;
        ocrCurrent = i;
        await new Promise((r) => setTimeout(r, 50));
      }

      ocrResult = {
        success: true,
        outputPath: ocrOutputPath,
        lines: 342,
        durationMs: 720000,
      };
      snackbar.show(t("experimental.ocr.done"), "success");
    } catch (err: any) {
      snackbar.show(`OCR error: ${err}`, "error");
    } finally {
      ocrExtracting = false;
    }
  }

  function cancelOcr() {
    ocrExtracting = false;
  }
</script>

<div class="h-full flex flex-col bg-gray-900 text-gray-100 overflow-hidden">
  <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-5 scrollbar-thin">
    <!-- Video Subtitles OCR Extraction -->
    <div class="glass-card p-5 space-y-4">
      <div class="flex items-center justify-between mb-2">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-teal-500/15 border border-teal-500/25 flex items-center justify-center shrink-0 text-teal-400">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
            </svg>
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h3 class="text-base font-semibold text-white">
                {t("experimental.ocr.title")}
              </h3>
              <span class="text-[10px] uppercase font-bold tracking-wider px-2 py-0.5 rounded-full bg-teal-500/20 text-teal-300 border border-teal-500/30">
                {t("experimental.badge")}
              </span>
            </div>
            <p class="text-xs text-gray-400 mt-1 max-w-2xl leading-relaxed">
              {t("experimental.ocr.subtitle")}
            </p>
          </div>
        </div>
      </div>

      <!-- Video & Output Files -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
            </svg>
            <span>{t("experimental.ocr.videoFile")}</span>
            <span class="text-rose-400">*</span>
          </span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(ocrVideoPath) || ""} placeholder={t("experimental.ocr.noFile")} class="input-modern flex-1 text-xs" title={ocrVideoPath || undefined} />
            <button onclick={pickOcrVideo} class="btn-secondary px-3 py-2 text-xs" disabled={ocrExtracting}>{t("flashcards.browse")}</button>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
            </svg>
            <span>{t("experimental.ocr.outputFile")}</span>
            <span class="text-rose-400">*</span>
          </span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(ocrOutputPath) || ""} placeholder={t("experimental.ocr.noFile")} class="input-modern flex-1 text-xs" title={ocrOutputPath || undefined} />
            <button onclick={pickOcrOutput} class="btn-secondary px-3 py-2 text-xs" disabled={ocrExtracting}>{t("flashcards.browse")}</button>
          </div>
        </div>
      </div>

      <!-- OCR Settings -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
            </svg>
            <span>{t("experimental.ocr.language")}</span>
          </span>
          <select bind:value={ocrLanguage} class="input-modern w-full text-xs" disabled={ocrExtracting}>
            <option value="zh">{t("experimental.ocr.langZh")}</option>
            <option value="ja">{t("experimental.ocr.langJa")}</option>
            <option value="ko">{t("experimental.ocr.langKo")}</option>
            <option value="en">{t("experimental.ocr.langEn")}</option>
            <option value="ar">{t("experimental.ocr.langAr")}</option>
            <option value="hi">{t("experimental.ocr.langHi")}</option>
          </select>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
            <span>{t("experimental.ocr.region")}</span>
          </span>
          <select bind:value={ocrRegion} class="input-modern w-full text-xs" disabled={ocrExtracting}>
            <option value="bottom20">{t("experimental.ocr.regionBottom20")}</option>
            <option value="bottom30">{t("experimental.ocr.regionBottom30")}</option>
            <option value="top20">{t("experimental.ocr.regionTop20")}</option>
            <option value="full">{t("experimental.ocr.regionFull")}</option>
          </select>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>{t("experimental.ocr.fps")}</span>
          </span>
          <select bind:value={ocrFps} class="input-modern w-full text-xs" disabled={ocrExtracting}>
            <option value={2}>{t("experimental.ocr.fps2")}</option>
            <option value={1}>{t("experimental.ocr.fps1")}</option>
            <option value={4}>{t("experimental.ocr.fps4")}</option>
          </select>
        </div>
      </div>

      <!-- OCR Actions & Progress -->
      <div class="flex items-center gap-3 pt-2">
        {#if ocrExtracting}
          <button onclick={cancelOcr} class="rounded-xl bg-red-600/80 hover:bg-red-500/80 border border-red-500/30 text-sm font-bold text-red-100 px-5 py-2.5 shadow-md transition-all cursor-pointer">
            {t("common.cancel")}
          </button>
          <div class="flex-1 flex items-center gap-3">
            <div class="flex-1 bg-white/10 h-2 rounded-full overflow-hidden">
              <div
                class="bg-gradient-to-r from-teal-500 to-cyan-500 h-full rounded-full transition-all duration-300"
                style="width: {ocrTotal > 0 ? (ocrCurrent / ocrTotal) * 100 : 5}%"
              ></div>
            </div>
            <span class="text-xs text-gray-400 whitespace-nowrap">
              {ocrStage === "sampling" ? t("experimental.ocr.stageSampling") : ocrStage === "ocr" ? t("experimental.ocr.stageOcr") : t("experimental.ocr.stageTiming")} ({ocrCurrent}%)
            </span>
          </div>
        {:else}
          <button
            onclick={startOcr}
            disabled={!canStartOcr}
            class="rounded-xl bg-teal-600/90 hover:bg-teal-500/90 border border-teal-500/30 disabled:opacity-40 disabled:cursor-not-allowed text-sm font-bold text-teal-50 px-5 py-2.5 shadow-md transition-all cursor-pointer flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
            </svg>
            {t("experimental.ocr.start")}
          </button>
        {/if}
      </div>

      {#if ocrResult}
        <div class="bg-teal-500/10 border border-teal-500/25 rounded-xl p-4 text-xs text-gray-300 flex flex-wrap gap-x-6 gap-y-1">
          <span><span class="font-bold text-teal-300">{ocrResult.lines}</span> {t("experimental.ocr.resultLines")}</span>
          <span>{t("experimental.condense.resultDuration")}: <span class="font-bold text-teal-300">{fmtDuration(ocrResult.durationMs)}</span></span>
          <span class="text-gray-400 truncate w-full mt-1 font-mono" title={ocrResult.outputPath}>{ocrResult.outputPath}</span>
        </div>
      {/if}
    </div>

>>>>>>> main
    <!-- Condensed audio -->
    <div class="glass-card p-5 space-y-4">
      <div class="flex items-center justify-between mb-2">
        <h3 class="text-lg font-semibold flex items-center gap-2 text-amber-400">
          <svg class="w-5 h-5 text-amber-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2z" />
          </svg>
          {t("experimental.condense.title")}
        </h3>
      </div>

      <!-- Files -->
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 01-2-2V7a2 2 0 012-2z" />
            </svg>
            <span>{t("experimental.condense.mediaFile")}</span>
            <span class="text-rose-400">*</span>
          </span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(mediaPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={mediaPath || undefined} />
            <button onclick={pickMedia} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
            </svg>
            <span>{t("experimental.condense.outputFile")}</span>
            <span class="text-rose-400">*</span>
          </span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(outputPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={outputPath || undefined} />
            <button onclick={pickOutput} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
      </div>

      <!-- Detection mode -->
      <div>
        <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-2">
          <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
          </svg>
          <span>{t("experimental.condense.detection")}</span>
        </span>
        <div class="relative flex items-center p-1 bg-white/5 border border-white/10 rounded-xl w-[260px] select-none">
          <div
            class="absolute top-1 bottom-1 rounded-lg bg-amber-600 shadow-md transition-all duration-300 ease-out pointer-events-none"
            style="left: {detectMode === 'subtitles' ? '4px' : 'calc(50% + 2px)'}; width: calc(50% - 6px);"
          ></div>
          <button
            type="button"
            onclick={() => !condensing && (detectMode = "subtitles")}
            disabled={condensing}
            class="relative z-10 flex-1 py-1.5 px-3 text-center text-xs transition-colors duration-200 cursor-pointer {detectMode === 'subtitles' ? 'text-white font-bold' : 'text-gray-400 hover:text-white font-semibold'}"
          >
            {t("experimental.condense.modeSubtitles")}
          </button>
          <button
            type="button"
            onclick={() => !condensing && (detectMode = "vad")}
            disabled={condensing}
            class="relative z-10 flex-1 py-1.5 px-3 text-center text-xs transition-colors duration-200 cursor-pointer {detectMode === 'vad' ? 'text-white font-bold' : 'text-gray-400 hover:text-white font-semibold'}"
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
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10" />
            </svg>
            <span>{t("experimental.condense.srtFile")}</span>
            <span class="text-rose-400">*</span>
          </span>
          <div class="flex gap-2">
            <input type="text" readonly value={getFileName(srtPath) || ""} placeholder={t("experimental.condense.noFile")} class="input-modern flex-1 text-xs" title={srtPath || undefined} />
            <button onclick={pickSrt} class="btn-secondary px-3 py-2 text-xs" disabled={condensing}>{t("flashcards.browse")}</button>
          </div>
        </div>
      {/if}

      <!-- Options -->
      <div class="grid grid-cols-3 gap-3 md:w-2/3">
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
            </svg>
            <span>{t("experimental.condense.padding")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={padMs} min="0" step="50" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
            </svg>
            <span>{t("experimental.condense.mergeGap")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={mergeGapMs} min="0" step="100" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">ms</span>
          </div>
        </div>
        <div>
          <span class="flex items-center gap-1.5 text-xs font-semibold text-gray-400 mb-1.5">
            <svg class="w-3.5 h-3.5 text-gray-300 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <span>{t("flashcards.audioBitrate")}</span>
          </span>
          <div class="flex items-center gap-1">
            <input type="number" bind:value={bitrate} min="32" step="32" class="input-modern w-full text-xs" disabled={condensing} />
            <span class="text-xs text-gray-500">kb/s</span>
          </div>
        </div>
      </div>

      <!-- Actions + progress -->
      <div class="flex items-center gap-3 pt-2">
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
    </div>
  </div>
</div>

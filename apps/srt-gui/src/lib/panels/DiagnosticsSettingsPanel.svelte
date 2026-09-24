<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale } from "$lib/i18n";
  import { displayComputeBackend, type SystemDiagnostics } from "$lib/services/systemDiagnostics";

  let t = $derived($locale);
  let diagnostics = $state<SystemDiagnostics | null>(null);
  let loading = $state(false);
  let error = $state(false);

  async function refresh() {
    loading = true;
    error = false;
    try {
      diagnostics = await invoke<SystemDiagnostics>("get_system_diagnostics");
    } catch (reason) {
      console.error("Unable to load system diagnostics", reason);
      error = true;
    } finally {
      loading = false;
    }
  }

  function status(value: boolean): string {
    return t(value ? "settings.endpointStatus.online" : "settings.endpointStatus.offline");
  }

  onMount(refresh);
</script>

<div class="space-y-5">
  <div class="flex items-start justify-between gap-4">
    <div>
      <h2 class="text-xl font-bold text-white">{t("settings.diagnostics.title")}</h2>
      <p class="mt-1 text-sm text-gray-400">{t("settings.diagnostics.description")}</p>
    </div>
    <button type="button" onclick={refresh} disabled={loading} class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-sm font-semibold text-gray-200 transition hover:bg-white/10 disabled:opacity-50 cursor-pointer">
      {t("settings.diagnostics.refresh")}
    </button>
  </div>

  {#if loading && !diagnostics}
    <div class="glass-card p-8 text-center text-sm text-gray-400">{t("settings.endpointStatus.checking")}</div>
  {:else if error || !diagnostics}
    <div class="glass-card border-red-500/20 p-8 text-center text-sm text-red-300">{t("settings.diagnostics.error")}</div>
  {:else}
    <div class="grid grid-cols-1 gap-5 xl:grid-cols-2">
      <section class="glass-card p-5">
        <h3 class="text-sm font-bold text-cyan-300">{t("settings.diagnostics.compute")}</h3>
        <div class="mt-4 rounded-xl border border-cyan-400/20 bg-cyan-500/10 p-4">
          <div class="text-xs uppercase tracking-wider text-gray-400">{t("settings.diagnostics.activeBackend")}</div>
          <div class="mt-1 text-lg font-bold text-white">{displayComputeBackend(diagnostics)}</div>
        </div>
        <p class="mt-3 text-xs text-gray-400">{diagnostics.gpu_compiled ? t("settings.diagnostics.gpuEnabled") : t("settings.diagnostics.cpuFallback")}</p>
      </section>

      <section class="glass-card p-5">
        <h3 class="text-sm font-bold text-emerald-300">{t("settings.diagnostics.media")}</h3>
        <dl class="mt-4 space-y-3 text-sm">
          <div class="flex justify-between gap-4"><dt class="text-gray-400">FFmpeg</dt><dd class={diagnostics.ffmpeg_available ? "text-emerald-300" : "text-red-300"}>{status(diagnostics.ffmpeg_available)}</dd></div>
          <div class="flex justify-between gap-4"><dt class="text-gray-400">{t("settings.diagnostics.videoEncoder")}</dt><dd class="text-right text-gray-200">{diagnostics.video_encoder}</dd></div>
        </dl>
      </section>

      <section class="glass-card p-5">
        <h3 class="text-sm font-bold text-violet-300">{t("settings.diagnostics.preview")}</h3>
        <dl class="mt-4 space-y-3 text-sm">
          <div class="flex justify-between gap-4"><dt class="text-gray-400">GStreamer</dt><dd class={diagnostics.gstreamer_available ? "text-emerald-300" : "text-red-300"}>{status(diagnostics.gstreamer_available)}</dd></div>
          <div class="flex justify-between gap-4"><dt class="text-gray-400">H.264</dt><dd class={diagnostics.gstreamer_h264 ? "text-emerald-300" : "text-red-300"}>{status(diagnostics.gstreamer_h264)}</dd></div>
          <div class="flex justify-between gap-4"><dt class="text-gray-400">H.265</dt><dd class={diagnostics.gstreamer_h265 ? "text-emerald-300" : "text-red-300"}>{status(diagnostics.gstreamer_h265)}</dd></div>
        </dl>
      </section>

      <section class="glass-card p-5">
        <h3 class="text-sm font-bold text-amber-300">{t("settings.diagnostics.system")}</h3>
        <div class="mt-4 text-lg font-semibold text-white">{diagnostics.os} · {diagnostics.arch}</div>
      </section>
    </div>
  {/if}
</div>

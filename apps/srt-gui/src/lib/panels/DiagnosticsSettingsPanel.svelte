<script lang="ts">
  import { onMount } from "svelte";
  import { locale } from "$lib/i18n";
  import { displayComputeBackend } from "$lib/services/systemDiagnostics";
  import { diagnosticsStore as store } from "$lib/stores/diagnosticsStore.svelte";
  import { supportLogStore } from "$lib/stores/supportLogStore.svelte";
  import SupportLogsPanel from "$lib/panels/SupportLogsPanel.svelte";
  let t = $derived($locale);
  function status(value: boolean | undefined): string {
    return value === undefined ? t("settings.endpointStatus.checking") : t(value ? "settings.endpointStatus.online" : "settings.endpointStatus.offline");
  }
  onMount(() => { void store.refresh(); void supportLogStore.initialize(); });
</script>
<div class="space-y-5">
  <div class="flex justify-end">
    <button type="button" onclick={() => store.refresh(true)} disabled={store.loading} class="btn-secondary px-4 py-2 text-sm disabled:opacity-50">{t("settings.diagnostics.refresh")}</button>
  </div>
  {#if store.error}<p class="text-sm text-red-300" role="alert">{t("settings.diagnostics.error")}</p>{/if}
  <div class="grid grid-cols-1 gap-5 xl:grid-cols-2" aria-busy={store.loading}>
    <section class="glass-card p-5 min-h-44">
      <h3 class="text-sm font-bold text-cyan-300">{t("settings.diagnostics.compute")}</h3>
      <div class="mt-4 rounded-xl border border-cyan-400/20 bg-cyan-500/10 p-4">
        <div class="text-xs uppercase tracking-wider text-gray-400">{t("settings.diagnostics.activeBackend")}</div>
        <div class="mt-1 text-lg font-bold text-white">{store.data ? displayComputeBackend(store.data) : '—'}</div>
      </div>
      <p class="mt-3 text-xs text-gray-400">{store.data ? t(store.data.gpu_compiled ? "settings.diagnostics.gpuEnabled" : "settings.diagnostics.cpuFallback") : t("settings.endpointStatus.checking")}</p>
    </section>
    <section class="glass-card p-5 min-h-44">
      <h3 class="text-sm font-bold text-emerald-300">{t("settings.diagnostics.media")}</h3>
      <dl class="mt-4 space-y-3 text-sm">
        <div class="flex justify-between gap-4"><dt class="text-gray-400">FFmpeg</dt><dd class={store.data?.ffmpeg_available ? 'text-emerald-300' : 'text-gray-400'}>{status(store.data?.ffmpeg_available)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">{t("settings.diagnostics.videoEncoder")}</dt><dd class="text-right text-gray-200">{store.data?.video_encoder ?? '—'}</dd></div>
      </dl>
    </section>
    <section class="glass-card p-5 min-h-44">
      <h3 class="text-sm font-bold text-violet-300">{t("settings.diagnostics.preview")}</h3>
      <dl class="mt-4 space-y-3 text-sm">
        <div class="flex justify-between gap-4"><dt class="text-gray-400">GStreamer</dt><dd class={store.data?.gstreamer_available ? 'text-emerald-300' : 'text-gray-400'}>{status(store.data?.gstreamer_available)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">H.264</dt><dd class="text-gray-200">{status(store.data?.gstreamer_h264)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">H.265</dt><dd class="text-gray-200">{status(store.data?.gstreamer_h265)}</dd></div>
      </dl>
    </section>
    <section class="glass-card p-5 min-h-44">
      <h3 class="text-sm font-bold text-amber-300">{t("settings.diagnostics.system")}</h3>
      <div class="mt-4 text-lg font-semibold text-white">{store.data ? `${store.data.os} · ${store.data.arch}` : '—'}</div>
    </section>
  </div>
  <SupportLogsPanel />
</div>

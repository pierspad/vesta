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
  onMount(() => { void store.refresh(true); void supportLogStore.initialize(); });
</script>
<div class="space-y-4">
  {#if store.error}<p class="text-sm text-red-300" role="alert">{t("settings.diagnostics.error")} <button type="button" onclick={() => store.refresh(true)} disabled={store.loading} class="ml-2 underline disabled:opacity-50">{t("settings.diagnostics.refresh")}</button></p>{/if}
  <div class="grid grid-cols-1 gap-3 lg:grid-cols-2 2xl:grid-cols-4" aria-busy={store.loading}>
    <section class="glass-card p-4">
      <h3 class="flex items-center gap-2 text-sm font-bold text-cyan-300"><svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7V4h3m10 0h3v3M4 17v3h3m10 0h3v-3M8 8h8v8H8zM12 1v3m0 16v3M1 12h3m16 0h3" /></svg>{t("settings.diagnostics.compute")}</h3>
      <div class="mt-3 rounded-xl border border-cyan-400/20 bg-cyan-500/10 p-3">
        <div class="text-xs uppercase tracking-wider text-gray-400">{t("settings.diagnostics.activeBackend")}</div>
        <div class="mt-1 text-sm font-bold text-white">{store.data ? displayComputeBackend(store.data) : '—'}</div>
      </div>
      <p class="mt-3 text-xs text-gray-400">{store.data ? t(store.data.gpu_compiled ? "settings.diagnostics.gpuEnabled" : "settings.diagnostics.cpuFallback") : t("settings.endpointStatus.checking")}</p>
    </section>
    <section class="glass-card p-4">
      <h3 class="flex items-center gap-2 text-sm font-bold text-emerald-300"><svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h16v16H4zM4 9h16M9 4v5m6-5v5m-5 4 5 3-5 3z" /></svg>{t("settings.diagnostics.media")}</h3>
      <dl class="mt-3 space-y-2 text-sm">
        <div class="flex justify-between gap-4"><dt class="text-gray-400">FFmpeg</dt><dd class={store.data?.ffmpeg_available ? 'text-emerald-300' : 'text-gray-400'}>{status(store.data?.ffmpeg_available)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">{t("settings.diagnostics.videoEncoder")}</dt><dd class="text-right text-gray-200">{store.data?.video_encoder ?? '—'}</dd></div>
      </dl>
    </section>
    <section class="glass-card p-4">
      <h3 class="flex items-center gap-2 text-sm font-bold text-violet-300"><svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 4h18v13H3zM8 21h8m-4-4v4m-2-13 5 3-5 3z" /></svg>{t("settings.diagnostics.preview")}</h3>
      <dl class="mt-3 space-y-2 text-sm">
        <div class="flex justify-between gap-4"><dt class="text-gray-400">GStreamer</dt><dd class={store.data?.gstreamer_available ? 'text-emerald-300' : 'text-gray-400'}>{status(store.data?.gstreamer_available)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">H.264</dt><dd class="text-gray-200">{status(store.data?.gstreamer_h264)}</dd></div>
        <div class="flex justify-between gap-4"><dt class="text-gray-400">H.265</dt><dd class="text-gray-200">{status(store.data?.gstreamer_h265)}</dd></div>
      </dl>
    </section>
    <section class="glass-card p-4">
      <h3 class="flex items-center gap-2 text-sm font-bold text-amber-300"><svg aria-hidden="true" class="h-5 w-5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4h16v16H4zM8 9l3 3-3 3m5 0h3" /></svg>{t("settings.diagnostics.system")}</h3>
      <div class="mt-3 text-sm font-semibold text-white">{store.data ? `${store.data.os} · ${store.data.arch}` : '—'}</div>
    </section>
  </div>
  <SupportLogsPanel />
</div>

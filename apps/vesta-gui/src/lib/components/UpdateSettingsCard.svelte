<script lang="ts">
  import { locale } from "$lib/i18n";
  import { updateCheckerStore as updates } from "$lib/stores/updateCheckerStore.svelte";
  import ToggleRow from "./ToggleRow.svelte";
  let t = $derived($locale);
  let status = $derived.by(() => {
    switch (updates.updateStatus) {
      case "available": return t("settings.updatesNewVersionAvailable", { version: updates.latestVersion });
      case "current": return t("settings.updatesUpToDate");
      case "checking": return t("settings.updatesChecking");
      case "error": return t("settings.updatesCheckFailed");
      case "offline": return t("settings.updatesOffline");
      default: return updates.automaticUpdateChecks ? t("settings.updatesActiveOnStartup") : "";
    }
  });
  let available = $derived(updates.updateStatus === "available");
  let installable = $derived(available && updates.canInstall && !updates.managerHint);
  let busy = $derived(updates.updateStatus === "checking" || updates.installing);
  let actionLabel = $derived(updates.installing
    ? t("settings.updatesDownloading", { percent: updates.downloadPercent })
    : installable ? t("settings.updatesInstall", { version: updates.latestVersion })
    : available ? t("settings.updatesReleaseDetails") : t("settings.updatesCheckNow"));

  function runAction() {
    if (busy) return;
    if (installable) void updates.installUpdate();
    else if (available) void updates.openRelease();
    else void updates.checkForUpdates("manual");
  }
</script>

<section class="glass-card flex flex-col gap-4 p-5" aria-label={t("settings.updatesStatus")}>
  <ToggleRow label={t("settings.updatesCheckOnStartup")}
    bind:checked={updates.automaticUpdateChecks}
    onchange={() => updates.onAutomaticUpdateChecksChange()} accent="indigo"
    iconPath="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10" />
  <div class="mt-auto flex flex-wrap items-center justify-between gap-3 border-t border-white/5 pt-4">
    <p role="status" class="min-w-0 flex-1 text-sm leading-relaxed {updates.updateStatus === 'current' ? 'text-emerald-300' : available ? 'text-indigo-200' : 'text-gray-400'}">{status}</p>
    <button type="button" onclick={runAction} disabled={busy}
      class="{available ? 'btn-primary' : 'btn-secondary'} shrink-0 px-4 py-2 text-xs disabled:opacity-50 disabled:cursor-not-allowed">{actionLabel}</button>
  </div>
  {#if available && updates.managerHint}
    <p class="text-xs leading-relaxed text-gray-400">{updates.managerHint}</p>
  {/if}
</section>

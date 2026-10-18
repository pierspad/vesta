<script lang="ts">
  import { locale } from "$lib/i18n";
  import * as vestaConfig from "$lib/config/vestaConfig";

  import FirstRunSetupModal from "$lib/modals/FirstRunSetupModal.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";
  let showReset = $state(false);
  let resetting = $state(false);
  let resetError = $state("");
  async function resetInstallation() {
    if (resetting) return;
    showReset = false;
    resetting = true;
    try {
      await vestaConfig.resetInstallation();
      window.location.reload();
    } catch (error) {
      resetError = String(error);
      resetting = false;
    }
  }
  let showPreview = $state(false);

  let t = $derived($locale);

  async function restartFirstRunSetup() {
    vestaConfig.setItem("vesta-first-run-force", "true");
    await vestaConfig.flush();
    window.location.reload();
  }
</script>

<div class="h-full flex flex-col bg-gray-900 text-gray-100 overflow-hidden">
  <div class="flex-1 overflow-y-auto p-6 scrollbar-thin">
    <div class="glass-card p-5 flex items-center justify-between gap-5">
      <div>
        <h3 class="text-base font-semibold text-white">{t("experimental.setup.title")}</h3>
        <p class="mt-1 text-xs text-gray-400">{t("experimental.setup.description")}</p>
      </div>
      <div class="flex flex-wrap gap-2">
      <button type="button" class="btn-secondary shrink-0 px-4 py-2 text-xs" onclick={() => showPreview = true}>{t("experimental.setup.try")}</button>
      <button
        type="button"
        class="btn-secondary shrink-0 px-4 py-2 text-xs"
        onclick={restartFirstRunSetup}
      >
        {t("experimental.setup.restart")}
      </button>
      </div>
    </div>
    <div class="glass-card p-5 mt-5">
      <h3 class="text-base font-semibold">{t("experimental.reset.title")}</h3>
      <p class="mt-1 mb-4 text-xs text-gray-400">{t("experimental.reset.description")}</p>
      <button type="button" class="btn-secondary px-4 py-2 text-xs" disabled={resetting} onclick={() => showReset = true}>{t("experimental.reset.action")}</button>
      {#if resetError}<p role="alert" class="mt-3 text-xs text-red-400">{resetError}</p>{/if}
    </div>
  </div>
</div>
<ConfirmDialog show={showReset} title={t("experimental.reset.title")} message={t("experimental.reset.description")} on:confirm={resetInstallation} on:cancel={() => showReset = false} />

{#if showPreview}
  <FirstRunSetupModal preview onClose={() => showPreview = false} onComplete={() => { showPreview = false; }} />
{/if}

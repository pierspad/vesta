<script lang="ts">
  import { locale } from "$lib/i18n";
  import * as vestaConfig from "$lib/config/vestaConfig";

  import FirstRunSetupModal from "$lib/modals/FirstRunSetupModal.svelte";
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
  </div>
</div>

{#if showPreview}
  <FirstRunSetupModal preview onClose={() => showPreview = false} onComplete={() => { showPreview = false; }} />
{/if}

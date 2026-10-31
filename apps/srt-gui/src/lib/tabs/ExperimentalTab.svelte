<script lang="ts">
  import { locale } from "$lib/i18n";
  import * as vestaConfig from "$lib/config/vestaConfig";

  import FirstRunSetupModal from "$lib/modals/FirstRunSetupModal.svelte";
  import ConfirmDialog from "$lib/modals/ConfirmDialog.svelte";

  interface Props {
    onGoToTab?: (tab: string) => void;
  }
  let { onGoToTab }: Props = $props();

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

  function hideExperimentalTab() {
    vestaConfig.setItem("vesta-experimental-unlocked", "false");
    window.dispatchEvent(new CustomEvent("vesta-experimental-toggled", { detail: { unlocked: false } }));
    if (onGoToTab) {
      onGoToTab("flashcards");
    } else if (typeof (window as any).changeTab === "function") {
      (window as any).changeTab("flashcards");
    }
  }
</script>

<div class="h-full flex flex-col bg-gray-900 text-gray-100 overflow-hidden">
  <div class="flex-1 overflow-y-auto p-6 scrollbar-thin space-y-5">
    <!-- Card 1: Hide Experimental Tab (at the top) -> Amber/Orange accent -->
    <div class="glass-card p-5 border border-amber-500/25 bg-amber-500/[0.04] flex items-center justify-between gap-5 rounded-2xl">
      <div class="flex items-start gap-4">
        <div class="p-2.5 rounded-xl bg-amber-500/15 text-amber-400 shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m14.41 14.41l-3.59-3.59"/></svg>
        </div>
        <div>
          <h3 class="text-base font-semibold text-white">{t("experimental.hide.title")}</h3>
          <p class="mt-1 text-xs text-gray-400">{t("experimental.hide.description")}</p>
        </div>
      </div>
      <button
        type="button"
        class="text-amber-400 border border-amber-500/30 bg-amber-500/15 hover:bg-amber-500/25 shrink-0 px-4 py-2 text-xs rounded-xl font-medium transition-colors cursor-pointer"
        onclick={hideExperimentalTab}
      >
        {t("experimental.hide.action")}
      </button>
    </div>

    <!-- Card 2: First-run setup preview -> Indigo/Cyan accent -->
    <div class="glass-card p-5 border border-indigo-500/25 bg-indigo-500/[0.04] flex items-center justify-between gap-5 rounded-2xl">
      <div class="flex items-start gap-4">
        <div class="p-2.5 rounded-xl bg-indigo-500/15 text-indigo-400 shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/></svg>
        </div>
        <div>
          <h3 class="text-base font-semibold text-white">{t("experimental.setup.title")}</h3>
          <p class="mt-1 text-xs text-gray-400">{t("experimental.setup.description")}</p>
        </div>
      </div>
      <div class="flex flex-wrap gap-2 shrink-0">
        <button type="button" class="text-indigo-300 border border-indigo-500/30 bg-indigo-500/15 hover:bg-indigo-500/25 px-3.5 py-2 text-xs rounded-xl font-medium transition-colors cursor-pointer" onclick={() => showPreview = true}>{t("experimental.setup.try")}</button>
        <button
          type="button"
          class="btn-secondary px-3.5 py-2 text-xs rounded-xl cursor-pointer"
          onclick={restartFirstRunSetup}
        >
          {t("experimental.setup.restart")}
        </button>
      </div>
    </div>

    <!-- Card 3: Simulate a fresh installation -> Rose/Red danger accent -->
    <div class="glass-card p-5 border border-rose-500/25 bg-rose-500/[0.04] flex items-center justify-between gap-5 rounded-2xl">
      <div class="flex items-start gap-4">
        <div class="p-2.5 rounded-xl bg-rose-500/15 text-rose-400 shrink-0">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
        </div>
        <div>
          <h3 class="text-base font-semibold text-white">{t("experimental.reset.title")}</h3>
          <p class="mt-1 mb-2 text-xs text-gray-400">{t("experimental.reset.description")}</p>
          {#if resetError}<p role="alert" class="text-xs text-rose-400">{resetError}</p>{/if}
        </div>
      </div>
      <button
        type="button"
        class="text-rose-300 border border-rose-500/30 bg-rose-500/15 hover:bg-rose-500/25 shrink-0 px-4 py-2 text-xs rounded-xl font-medium transition-colors cursor-pointer"
        disabled={resetting}
        onclick={() => showReset = true}
      >
        {t("experimental.reset.action")}
      </button>
    </div>
  </div>
</div>
<ConfirmDialog show={showReset} title={t("experimental.reset.title")} message={t("experimental.reset.description")} on:confirm={resetInstallation} on:cancel={() => showReset = false} />

{#if showPreview}
  <FirstRunSetupModal preview onClose={() => showPreview = false} onComplete={() => { showPreview = false; }} />
{/if}

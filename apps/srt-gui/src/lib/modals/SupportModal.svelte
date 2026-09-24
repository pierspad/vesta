<script lang="ts">
  import { locale } from "$lib/i18n";
  import { supportLinks } from "$lib/config/supportLinks";
  import SponsorIcon from "$lib/components/SponsorIcon.svelte";

  let { show, onclose }: { show: boolean; onclose: () => void } = $props();
  let t = $derived($locale);

  const cardStyles: Record<string, { hover: string; arrowHover: string }> = {
    github: {
      hover: "hover:border-pink-400/40 hover:bg-pink-500/10",
      arrowHover: "group-hover:text-pink-300",
    },
    coffee: {
      hover: "hover:border-amber-400/40 hover:bg-amber-500/10",
      arrowHover: "group-hover:text-amber-300",
    },
    kofi: {
      hover: "hover:border-rose-400/40 hover:bg-rose-500/10",
      arrowHover: "group-hover:text-rose-300",
    },
  };
</script>

<svelte:window onkeydown={(event) => show && event.key === "Escape" && onclose()} />
{#if show}
  <div class="fixed inset-0 z-[200] flex items-center justify-center p-6">
    <button type="button" class="absolute inset-0 bg-black/75 backdrop-blur-sm" aria-label={t("common.close")} onclick={onclose}></button>
    <div class="relative w-full max-w-2xl rounded-2xl border border-pink-400/20 bg-gray-900 p-6 shadow-2xl" role="dialog" aria-modal="true" aria-labelledby="support-title" tabindex="-1">
      <div class="flex items-start justify-between gap-4">
        <div><h2 id="support-title" class="text-xl font-bold text-white">{t("support.title")}</h2><p class="mt-2 max-w-xl text-sm leading-relaxed text-gray-400">{t("support.description")}</p></div>
        <button class="rounded-lg p-2 text-gray-400 hover:bg-white/10 hover:text-white" aria-label={t("common.close")} onclick={onclose}>✕</button>
      </div>
      <div class="mt-6 grid gap-3 sm:grid-cols-3">
        {#each supportLinks as provider}
          {@const style = cardStyles[provider.id] ?? {
            hover: "hover:border-pink-400/40 hover:bg-pink-500/10",
            arrowHover: "group-hover:text-pink-300",
          }}
          <a
            href={provider.url}
            target="_blank"
            rel="noreferrer"
            class="group flex flex-col justify-between rounded-xl border border-white/10 bg-white/[0.04] p-4 text-left transition duration-200 hover:-translate-y-0.5 {style.hover}"
          >
            <div>
              <div class="mb-3 flex items-center justify-between">
                <SponsorIcon sponsor={provider.id} />
                <svg
                  class="h-4 w-4 text-gray-500 transition-colors duration-200 {style.arrowHover}"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </div>
              <span class="text-sm font-bold text-white transition-colors duration-200">{provider.label}</span>
              <span class="mt-1.5 block text-xs leading-relaxed text-gray-400">{t(`support.${provider.id}`)}</span>
            </div>
          </a>
        {/each}
      </div>
      <div class="mt-5 flex justify-end"><button class="btn-secondary px-4 py-2" onclick={onclose}>{t("support.notNow")}</button></div>
    </div>
  </div>
{/if}

<script lang="ts">
  import { locale } from "$lib/i18n";
  import MatchFileEvidence from "$lib/components/MatchFileEvidence.svelte";
  import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
  let { episode, focusedPath = "", onclose, onrules }: {
    episode: EpisodeEntry; focusedPath?: string; onclose: () => void; onrules: () => void;
  } = $props();
  let t = $derived($locale);
  let dialog: HTMLDialogElement;
  $effect(() => { dialog.showModal(); });
  const fields = [
    { field: "targetSubsPath", label: "flashcards.targetLangSubs" },
    { field: "nativeSubsPath", label: "flashcards.nativeLangSubs" },
    { field: "mediaPath", label: "flashcards.mediaFile" },
  ] as const;
  let activePaths = $derived(fields.map(item => episode[item.field]).filter(Boolean));
  let method = $derived(activePaths.length < 2 ? "unmatched"
    : activePaths.every(path => episode.matching?.files.some(file => file.path === path))
      ? episode.matching?.method : undefined);
</script>

<dialog bind:this={dialog} oncancel={(event) => { event.preventDefault(); onclose(); }}
  aria-label={t("matching.title")}
  class="m-auto w-[min(820px,92vw)] max-h-[85vh] overflow-y-auto rounded-2xl border border-white/15 bg-gray-900 p-6 text-gray-100 shadow-2xl backdrop:bg-black/70">
  <header class="mb-5 flex items-center justify-between gap-4">
    <h2 class="flex items-center gap-3 text-lg font-bold">
      <span class="flex h-9 w-9 items-center justify-center rounded-lg bg-cyan-500/15 text-cyan-300">
        <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="m10 13 4-4m-7 6-2 2a3 3 0 0 0 4 4l5-5a3 3 0 0 0 0-4M17 9l2-2a3 3 0 0 0-4-4l-5 5a3 3 0 0 0 0 4"/></svg>
      </span>
      {t("matching.title")}
    </h2>
    <button type="button" class="rounded-lg p-2 text-gray-400 hover:bg-white/5 hover:text-white" onclick={onclose} aria-label={t("common.close")}>✕</button>
  </header>
  {#if method === "baseKey"}
    <div class="mb-4 rounded-xl border border-emerald-400/20 bg-emerald-500/[0.06] p-4">
      <p class="text-sm font-semibold text-emerald-300">{t("matching.byName")}</p>
      <p class="mt-1 text-xs text-gray-400">{t("matching.noEpisodeRegex")}</p>
      <code class="mt-3 block break-all text-sm text-emerald-100">{episode.matching?.files[0]?.baseKey}</code>
    </div>
  {:else if method === "episode"}
    <p class="mb-4 text-sm font-semibold text-cyan-300">{t("matching.byEpisode")}</p>
  {:else}
    <p class="mb-4 text-sm text-gray-400">{t(method === "unmatched" ? "matching.singleFile" : "matching.notRecorded")}</p>
  {/if}
  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    {#each fields as item}
      {@const path = episode[item.field]}
      {#if path}
        <MatchFileEvidence {path} label={t(item.label)}
          evidence={episode.matching?.files.find(file => file.path === path)}
          episodeMatch={method === "episode"} focused={path === focusedPath} />
      {/if}
    {/each}
  </div>
  <footer class="mt-5 flex justify-end gap-3">
    <button type="button" class="btn-secondary px-4 py-2 text-sm" onclick={onrules}>{t("matching.editRules")}</button>
    <button type="button" class="btn-secondary px-4 py-2 text-sm" onclick={onclose}>{t("common.close")}</button>
  </footer>
</dialog>

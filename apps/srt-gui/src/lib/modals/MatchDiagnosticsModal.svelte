<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import type { EpisodeEntry } from "$lib/utils/seriesFileMatching";
  let { episode, onclose, onrules }: {
    episode: EpisodeEntry;
    onclose: () => void;
    onrules: () => void;
  } = $props();
  let t = $derived($locale);
  let dialog: HTMLDialogElement;
  $effect(() => { dialog.showModal(); });
  const fields = ["targetSubsPath", "nativeSubsPath", "mediaPath"] as const;
</script>

<dialog bind:this={dialog} oncancel={(event) => { event.preventDefault(); onclose(); }}
  aria-label={t("matching.inspect")}
  class="m-auto w-[min(850px,92vw)] max-h-[85vh] overflow-y-auto rounded-2xl border border-white/15 bg-gray-900 p-6 text-gray-100 shadow-2xl backdrop:bg-black/70">
  <h2 class="text-lg font-bold text-cyan-300">{t("matching.inspect")}</h2>
  <p class="my-3 text-sm text-gray-400">{t("matching.help")}</p>
  <div class="space-y-3">
    {#each fields as field}
      {@const path = episode[field]}
      {@const evidence = episode.matching?.files.find(file => file.path === path)}
      {#if path}
        <section class="rounded-xl border border-white/10 bg-white/5 p-4">
          <h3 class="break-all font-semibold text-sm" title={path}>{getFileName(path)}</h3>
          {#if evidence}
            <dl class="mt-3 space-y-2 break-words text-xs">
              <div><dt class="text-gray-400">baseKey</dt><dd><code>{evidence.baseKey}</code></dd></div>
              {#if evidence.episodeRule}
                <div>
                  <dt class="text-gray-400">episodeRegexes[{evidence.episodeRule.ruleIndex}] → {evidence.episodeRule.episode}</dt>
                  <dd class="select-text whitespace-pre-wrap break-all font-mono text-cyan-200">{evidence.episodeRule.pattern}</dd>
                  <dd class="font-mono">{evidence.episodeRule.matched}</dd>
                </div>
              {/if}
              {#if evidence.language}<div><dt class="text-gray-400">{t("transcribe.sourceLanguage")}</dt><dd>{evidence.language}</dd></div>{/if}
              {#if evidence.hints.length}<div><dt class="text-gray-400">originalSubtitleHints / referenceSubtitleHints</dt><dd>{evidence.hints.join(", ")}</dd></div>{/if}
              {#if evidence.removedTokens.length}<div><dt class="text-gray-400">removableNameTokens</dt><dd>{evidence.removedTokens.join(", ")}</dd></div>{/if}
            </dl>
          {:else}
            <p class="mt-2 text-xs text-amber-300">{t("matching.notRecorded")}</p>
          {/if}
        </section>
      {/if}
    {/each}
  </div>
  <div class="mt-5 flex justify-end gap-3">
    <button type="button" class="btn-secondary px-4 py-2 text-sm" onclick={onrules}>{t("settings.smartMatching.title")} · {t("common.edit")}</button>
    <button type="button" class="btn-secondary px-4 py-2 text-sm" onclick={onclose}>{t("common.close")}</button>
  </div>
</dialog>

<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { writeClipboard } from "$lib/utils/clipboard";
  import type { FileMatchEvidence } from "$lib/utils/fileMatchEvidence";
  let { path, label, evidence, episodeMatch = false, focused = false }: {
    path: string; label: string; evidence?: FileMatchEvidence; episodeMatch?: boolean; focused?: boolean;
  } = $props();
  let t = $derived($locale);
  let filename = $derived(getFileName(path));
  let captured = $derived(evidence?.episodeRule?.matched.trim() || "");
  let captureAt = $derived(captured ? filename.indexOf(captured) : -1);
</script>

<section class="min-w-0 rounded-xl border bg-white/[0.035] p-4 {focused ? 'border-violet-400/60 ring-1 ring-violet-400/15' : 'border-white/10'}" data-focused={focused}>
  <p class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-gray-500">{label}</p>
  <h3 class="break-all text-sm font-semibold leading-relaxed text-gray-200" title={path}>
    {#if episodeMatch && captureAt >= 0}
      {filename.slice(0, captureAt)}<mark class="rounded bg-cyan-400/20 px-0.5 text-cyan-200">{captured}</mark>{filename.slice(captureAt + captured.length)}
    {:else}{filename}{/if}
  </h3>
  {#if evidence}
    {#if episodeMatch && evidence.episodeRule}
      <div class="mt-3 rounded-lg border border-cyan-400/25 bg-cyan-950/25 p-3">
        <div class="mb-2 flex items-center justify-between gap-2">
          <p class="text-xs font-semibold text-cyan-300">{t("matching.appliedRule", { number: evidence.episodeRule.ruleIndex + 1 })}</p>
          <button type="button" class="text-xs text-gray-400 hover:text-white" title={t("common.copy")} aria-label={t("common.copy")} onclick={() => writeClipboard(evidence!.episodeRule!.pattern)}>
            <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V4H4v12h4"/></svg>
          </button>
        </div>
        <code class="block select-text whitespace-pre-wrap break-all text-xs text-cyan-100">{evidence.episodeRule.pattern}</code>
        <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
          <span class="text-gray-400">{t("matching.matchedText")}</span>
          <code class="rounded bg-white/5 px-2 py-1">{captured}</code>
          <span aria-hidden="true">→</span><span class="font-semibold text-cyan-200">{evidence.episodeRule.episode}</span>
        </div>
      </div>
    {/if}
    <details class="mt-3 text-xs">
      <summary class="cursor-pointer text-gray-400 hover:text-gray-200">{t("matching.details")}</summary>
      <dl class="mt-3 space-y-2 break-words border-t border-white/5 pt-3">
        <div><dt class="text-gray-500">{t("matching.normalizedName")}</dt><dd class="font-mono text-gray-300">{evidence.baseKey}</dd></div>
        {#if evidence.language}<div><dt class="text-gray-500">{t("transcribe.sourceLanguage")}</dt><dd>{evidence.language}</dd></div>{/if}
        {#if evidence.hints.length}<div><dt class="text-gray-500">{t("matching.roleHints")}</dt><dd>{evidence.hints.join(", ")}</dd></div>{/if}
        {#if evidence.removedTokens.length}<div><dt class="text-gray-500">{t("matching.ignoredTokens")}</dt><dd>{evidence.removedTokens.join(", ")}</dd></div>{/if}
      </dl>
    </details>
  {:else}
    <p class="mt-3 text-xs text-amber-300">{t("matching.notRecorded")}</p>
  {/if}
</section>

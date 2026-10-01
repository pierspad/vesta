<script lang="ts">
  import { locale } from "$lib/i18n";
  import { languageFlagUrl } from "$lib/config/languages";
  import type { SubtitleLanguageGroup, SubtitleTrack } from "$lib/utils/subtitleTracks";
  let { group, busy, ondownload, onpreview }: { group: SubtitleLanguageGroup; busy: boolean; ondownload: (track: SubtitleTrack) => void; onpreview: (track: SubtitleTrack) => void } = $props();
  let t = $derived($locale);
  let selectedIndex = $state<number | null>(null);
  let track = $derived(group.tracks.find(track => track.index === selectedIndex) ?? group.tracks[0]);
  let contextOpen = $state(false);
  $effect(() => { group.key; selectedIndex = null; contextOpen = false; });
</script>
<div class="relative flex min-h-[108px] items-center gap-3 rounded-xl border border-white/10 bg-black/15 p-4 transition-colors hover:border-teal-400/30 hover:bg-teal-500/[0.06] focus-within:border-teal-400/40 focus-within:bg-teal-500/[0.08]"
  ondblclick={() => { if (!busy && track.text_based) onpreview(track); }}
  oncontextmenu={(event) => { event.preventDefault(); contextOpen = !contextOpen; }}
  role="group" aria-label={group.languageName}>
  <div class="min-w-0 flex-1">
    <p class="flex items-center gap-2 text-sm font-semibold">
      {#if group.code}<img src={languageFlagUrl(group.code) ?? ""} alt="" class="h-4 w-6 shrink-0 rounded-sm" />{/if}
      <span>{group.languageName} {group.nativeName && group.nativeName !== group.languageName ? `· ${group.nativeName}` : ""}</span>
      {#if group.tracks.length > 1}<span class="rounded bg-white/5 px-1.5 text-xs text-gray-400">{group.tracks.length}</span>{/if}
    </p>
    {#if group.tracks.length > 1}
      <select class="mt-2 w-full rounded-lg border border-white/10 bg-gray-900 px-2 py-1.5 text-xs text-gray-200 focus:border-teal-400 outline-none" aria-label={t("extract.variant")} disabled={busy} value={track.index} onchange={(event) => selectedIndex = Number(event.currentTarget.value)}>
        {#each group.tracks as variant (variant.index)}<option value={variant.index}>#{variant.index} · {variant.title || variant.codec}{variant.text_based ? "" : ` · ${t("extract.ocr")}`}</option>{/each}
      </select>
    {:else}<p class="mt-2 break-words text-xs text-gray-400">#{track.index} {track.title ? `· ${track.title}` : ""}</p>{/if}
    <p class="mt-1 text-xs text-gray-400">{track.codec}{track.text_based ? "" : ` · ${t("extract.ocr")}`}</p>
    {#if track.languageConflict}<p class="mt-1 text-xs text-amber-300">{t("extract.languageConflict")} · {track.language}</p>{/if}
  </div>
  <div class="flex shrink-0 gap-2">
    <button class="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-white/10 text-gray-300 hover:bg-white/10 disabled:opacity-30" title={t("extract.preview")} aria-label={`${t("extract.preview")} · ${group.languageName} · #${track.index}`} disabled={busy || !track.text_based} onclick={() => onpreview(track)}>
      <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/></svg>
    </button>
    <button class="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-teal-500/20 bg-teal-500/10 text-teal-300 hover:bg-teal-500/25 disabled:cursor-not-allowed disabled:border-white/5 disabled:bg-white/5 disabled:text-gray-600" title={track.text_based ? t("extract.save") : t("extract.ocr")} aria-label={`${t("extract.save")} · ${group.languageName} · #${track.index}`} disabled={busy || !track.text_based} onclick={() => ondownload(track)}>
      <svg aria-hidden="true" class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m-5-5 5 5 5-5M5 16v4h14v-4" /></svg>
    </button>
  </div>
  {#if contextOpen}
    <div class="absolute right-4 top-full z-20 rounded-lg border border-white/10 bg-gray-900 p-1 shadow-xl" role="menu">
      <button role="menuitem" class="rounded px-3 py-2 text-sm hover:bg-white/10 disabled:opacity-30" disabled={busy || !track.text_based} onclick={() => { contextOpen = false; onpreview(track); }}>{t("extract.preview")}</button>
      <button role="menuitem" class="rounded px-2 py-2 text-sm text-gray-400 hover:bg-white/10" onclick={() => contextOpen = false}>{t("common.cancel")}</button>
    </div>
  {/if}
</div>

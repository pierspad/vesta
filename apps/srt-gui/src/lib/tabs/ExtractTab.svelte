<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { guardedOpen, guardedSave } from "$lib/utils/dialogGuard";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import { prepareSubtitleTracks, filterPreparedSubtitleTracks, subtitleOutputName, groupSubtitleTracks, SUBTITLE_PAGE_SIZE } from "$lib/utils/subtitleTracks";
  import SubtitlePreviewModal from "$lib/modals/SubtitlePreviewModal.svelte";
  import SubtitleTrackCard from "$lib/components/SubtitleTrackCard.svelte";
  import { locale, currentLanguage } from "$lib/i18n";
  let t = $derived($locale);
  interface Track { index: number; codec: string; language: string; title: string; text_based: boolean }
  let tracks = $state<Track[]>([]);
  let search = $state("");
  let page = $state(0);
  let prepared = $derived(prepareSubtitleTracks(tracks, $currentLanguage));
  let filtered = $derived(filterPreparedSubtitleTracks(prepared, search));
  let groups = $derived(groupSubtitleTracks(filtered, $currentLanguage));
  let pageCount = $derived(Math.max(1, Math.ceil(groups.length / SUBTITLE_PAGE_SIZE)));
  let visible = $derived(groups.slice(page * SUBTITLE_PAGE_SIZE, (page + 1) * SUBTITLE_PAGE_SIZE));
  $effect(() => { if (page >= pageCount) page = pageCount - 1; });
  let previewTrack = $state<Track | null>(null);
  let previewText = $state("");
  let previewError = $state("");
  const previews = new Map<number, string>();
  async function preview(track: Track) {
    if (busy || !track.text_based) return;
    previewTrack = track; previewText = ""; previewError = ""; busy = true;
    try {
      let text = previews.get(track.index);
      if (text === undefined) {
        text = await invoke<string>("preview_embedded_subtitle", { path, index: track.index });
        if (previews.size >= 16) previews.delete(previews.keys().next().value!);
        previews.set(track.index, text);
      }
      previewText = text;
    } catch (e) { previewError = String(e); }
    finally { busy = false; }
  }
  let path = $state("");
  let busy = $state(false);
  let error = $state("");
  let saved = $state("");
  async function chooseMedia() {
    if (busy) return;
    busy = true; error = "";
    try {
      const selected = await guardedOpen({ multiple: false, filters: [{ name: t("extract.media"), extensions: ["mkv", "mp4", "m4v", "avi", "mov", "webm", "ts"] }] });
      if (typeof selected !== "string") return;
      previews.clear(); previewTrack = null; path = selected; tracks = []; saved = ""; search = ""; page = 0;
      tracks = await invoke<Track[]>("embedded_subtitle_tracks", { path });
    }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function extract(track: Track) {
    if (busy) return;
    error = ""; saved = "";
    busy = true;
    try {
      const outputPath = await guardedSave({ defaultPath: subtitleOutputName(path, track), filters: [{ name: "SRT", extensions: ["srt"] }] });
      if (!outputPath) return;
      await invoke("extract_embedded_subtitle", { path, index: track.index, outputPath });
      saved = outputPath;
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function openFolder() {
    try { await invoke("open_output_path", { path: saved, folder: true }); }
    catch (e) { error = String(e); }
  }
</script>
<div class="h-full overflow-y-auto bg-gray-900 p-6 text-gray-100 scrollbar-thin">
  <div class="glass-card p-5">
    <h2 class="mb-4 text-lg font-semibold text-teal-300">{t("extract.title")}</h2>
    <PathPickerField label={t("flashcards.mediaFile")} value={path} placeholder={t("extract.choose")}
      browseTitle={t("flashcards.browse")} browseLabel={t("flashcards.browse")} onbrowse={chooseMedia}
      disabled={busy} onclear={() => { previews.clear(); previewTrack = null; path = ""; tracks = []; saved = ""; error = ""; search = ""; page = 0; }} />
    {#if error || busy || (path && tracks.length === 0)}
    <div class="mt-3 text-sm" role="status" aria-live="polite">
      {#if error}<span class="text-red-300">{error}</span>
      {:else if busy}<span class="text-teal-300">{t("extract.working")}</span>
      {:else if path && tracks.length === 0}<span class="text-gray-400">{t("extract.empty")}</span>{/if}
    </div>
    {/if}
      <div class="mt-4 flex flex-wrap items-center justify-between gap-3">
        <div class="relative w-full sm:w-80">
          <svg aria-hidden="true" class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 4 4" /></svg>
        <input disabled={busy || tracks.length === 0} type="search" value={search} oninput={(event) => { search = event.currentTarget.value; page = 0; }} aria-label={t("extract.search")} placeholder={t("extract.search")} class="w-full rounded-lg border border-white/10 bg-black/30 pl-10 pr-4 py-2.5 text-sm outline-none focus:border-teal-500 focus:ring-2 focus:ring-teal-500/20 disabled:opacity-50" />
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <span class="mr-2 text-xs text-gray-400">{groups.length} · {filtered.length} / {tracks.length}</span>
          <button class="btn-secondary inline-flex items-center gap-1.5 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-30" disabled={busy || tracks.length === 0 || page === 0} onclick={() => page -= 1}>
            <svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m15 6-6 6 6 6" /></svg>{t("extract.previous")}
          </button>
          <span class="min-w-10 text-center text-xs text-gray-400" aria-live="polite">{page + 1} / {pageCount}</span>
          <button class="btn-secondary inline-flex items-center gap-1.5 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-30" disabled={busy || tracks.length === 0 || page + 1 >= pageCount} onclick={() => page += 1}>
            {t("extract.next")}<svg aria-hidden="true" class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 6 6 6-6 6" /></svg>
          </button>
        </div>
      </div>
      <div class="mt-4 grid min-h-[588px] grid-cols-1 lg:grid-cols-2 content-start gap-3" aria-busy={busy}>
        {#each visible as group (group.key)}
          <SubtitleTrackCard {group} {busy} ondownload={extract} onpreview={preview} />
        {/each}
        {#if visible.length === 0}
          {#each Array(SUBTITLE_PAGE_SIZE) as _, index (index)}
            <div aria-hidden="true" class="flex min-h-[108px] items-center justify-between rounded-xl border border-white/5 bg-black/10 p-4 {busy ? 'animate-pulse motion-reduce:animate-none' : ''}">
              <div class="space-y-2"><div class="h-3 w-36 rounded bg-white/5"></div><div class="h-2 w-24 rounded bg-white/[0.03]"></div></div>
              <div class="h-10 w-10 rounded-lg border border-white/5"></div>
            </div>
          {/each}
        {/if}
      </div>
      {#if tracks.length > 0 && filtered.length === 0}<p class="mt-4 text-sm text-gray-400" role="status">{t("extract.noMatches")}</p>{/if}
    {#if saved}<div class="mt-4 flex items-center gap-3 text-sm text-emerald-300" role="status"><span class="break-all">{t("extract.saved")} {saved}</span><button class="btn-secondary shrink-0 px-3 py-2" onclick={openFolder}>{t("common.openOutputFolder")}</button></div>{/if}
  </div>
</div>

{#if previewTrack}
  <SubtitlePreviewModal title={`${t("extract.preview")} · #${previewTrack.index} · ${previewTrack.title || previewTrack.language}`} text={previewText} error={previewError} loading={busy} onclose={() => previewTrack = null} />
{/if}

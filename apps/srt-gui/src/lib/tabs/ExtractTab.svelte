<script lang="ts">
  import EmptyStatusLabel from "$lib/components/EmptyStatusLabel.svelte";
  import LoadingSpinner from "$lib/components/LoadingSpinner.svelte";
  import { setupWebviewDragDrop } from "$lib/utils/dragDrop";
  import { onMount, onDestroy } from "svelte";
  import { SubtitlePreviewSession } from "$lib/utils/subtitlePreviewSession";
  import { join } from "@tauri-apps/api/path";
  import { defaultOutputDirectory } from "$lib/utils/defaultOutputDirectory";
  import { snackbar } from "$lib/stores/snackbarStore.svelte";
  import { invokeCommand as invoke } from "$lib/services/tauriClient";
  import { guardedOpen } from "$lib/utils/dialogGuard";
  import PathPickerField from "$lib/components/PathPickerField.svelte";
  import { prepareSubtitleTracks, filterPreparedSubtitleTracks, subtitleOutputName, groupSubtitleTracks, SUBTITLE_PAGE_SIZE } from "$lib/utils/subtitleTracks";
  import SubtitlePreviewModal from "$lib/modals/SubtitlePreviewModal.svelte";
  import SubtitleTrackCard from "$lib/components/SubtitleTrackCard.svelte";
  import { locale, currentLanguage } from "$lib/i18n";
  let { active = false }: { active?: boolean } = $props();
  let isDraggingOver = $state(false);
  $effect(() => {
    if (!active) { isDraggingOver = false; return; }
    return setupWebviewDragDrop({
      isActive: () => active && !busy && downloadingIndex === null && !previewTrack,
      setDraggingOver: value => isDraggingOver = value,
      onDrop: paths => {
        const media = paths.find(value => /\.(mkv|mp4|m4v|avi|mov|webm|ts|mp3|flac|m4a|wav|ogg|aac)$/i.test(value));
        if (media) void loadMedia(media);
      },
    });
  });
  let t = $derived($locale);
  interface Track { index: number; codec: string; language: string; title: string; text_based: boolean }
  let tracks = $state<Track[]>([]);
  let tracksScanned = $state(false);
  let search = $state("");
  let page = $state(0);
  let prepared = $derived(prepareSubtitleTracks(tracks, $currentLanguage));
  let filtered = $derived(filterPreparedSubtitleTracks(prepared, search));
  let groups = $derived(groupSubtitleTracks(filtered, $currentLanguage));
  let pageCount = $derived(Math.max(1, Math.ceil(groups.length / SUBTITLE_PAGE_SIZE)));
  let visible = $derived(groups.slice(page * SUBTITLE_PAGE_SIZE, (page + 1) * SUBTITLE_PAGE_SIZE));
  $effect(() => { if (page >= pageCount) page = pageCount - 1; });
  let outputDir = $state("");
  let downloaded = $state<number[]>([]);
  let selections = $state<Record<string, number>>({});
  let previewGroups = $derived(groupSubtitleTracks(prepared, $currentLanguage));
  let previewVariants = $derived(previewGroups.find(group => group.tracks.some(track => track.index === previewTrack?.index))?.tracks.filter(track => track.text_based) ?? []);
  let previewPosition = $derived(previewVariants.findIndex(track => track.index === previewTrack?.index));
  onMount(() => {
    void defaultOutputDirectory().then(value => { if (!outputDir) { outputDir = value; void refreshDownloaded(); } }).catch(e => { error = String(e); });
    const refresh = () => { void refreshDownloaded(); };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  });
  async function refreshDownloaded() {
    const directory = outputDir;
    const media = path;
    const candidates = tracks.filter(track => track.text_based);
    if (!directory || !media) { downloaded = []; return; }
    try {
      const names = await invoke<string[]>("existing_subtitle_outputs", { directory, names: candidates.map(track => subtitleOutputName(media, track)) });
      if (directory === outputDir && media === path) downloaded = candidates.filter(track => names.includes(subtitleOutputName(media, track))).map(track => track.index);
    } catch (e) { error = String(e); }
  }
  async function chooseOutput() {
    if (busy || downloadingIndex !== null) return;
    busy = true;
    try {
      const selected = await guardedOpen({ directory: true, multiple: false, defaultPath: outputDir || undefined });
      if (typeof selected === "string") { outputDir = selected; downloaded = []; await refreshDownloaded(); }
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
  function selectPreview() {
    const group = previewGroups.find(group => group.tracks.some(track => track.index === previewTrack?.index));
    if (group && previewTrack) selections = { ...selections, [group.key]: previewTrack.index };
    closePreview();
  }
  let previewTrack = $state<Track | null>(null);
  let previewText = $state("");
  let previewError = $state("");
  let previewLoading = $state(false);
  let previewSession: SubtitlePreviewSession | null = null;
  let previewRequest = 0;
  function closePreview() {
    previewRequest += 1;
    previewSession?.close();
    previewSession = null;
    previewTrack = null;
    previewText = "";
    previewError = "";
    previewLoading = false;
  }
  onDestroy(closePreview);
  async function preview(track: Track) {
    if (busy || !track?.text_based) return;
    if (!previewSession) {
      const media = path;
      const variants = previewGroups.find(group => group.tracks.some(variant => variant.index === track.index))?.tracks.filter(variant => variant.text_based) ?? [track];
      previewSession = new SubtitlePreviewSession(variants.map(variant => variant.index), index => invoke<string>("preview_embedded_subtitle", { path: media, index }));
    }
    const session = previewSession;
    const request = ++previewRequest;
    previewTrack = track; previewText = ""; previewError = ""; previewLoading = true;
    try {
      const text = await session.select(track.index);
      if (request === previewRequest) previewText = text;
    } catch (e) { if (request === previewRequest) previewError = String(e); }
    finally { if (request === previewRequest) previewLoading = false; }
  }
  let path = $state("");
  let busy = $state(false);
  let downloadingIndex = $state<number | null>(null);
  let error = $state("");
  async function chooseMedia() {
    if (busy || downloadingIndex !== null) return;
    busy = true; error = "";
    try {
      const selected = await guardedOpen({ multiple: false, filters: [{ name: t("extract.media"), extensions: ["mkv", "mp4", "m4v", "avi", "mov", "webm", "ts"] }] });
      if (typeof selected !== "string") return;
      await loadMedia(selected, true);
    }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function loadMedia(selected: string, fromPicker = false) {
    if ((!fromPicker && busy) || downloadingIndex !== null) return;
    busy = true; error = ""; isDraggingOver = false;
    closePreview(); selections = {}; downloaded = []; path = selected; tracks = []; tracksScanned = false; search = ""; page = 0;
    try {
      tracks = await invoke<Track[]>("embedded_subtitle_tracks", { path: selected });
      tracksScanned = true;
      await refreshDownloaded();
    } catch (e) { error = String(e); }
    finally { busy = false; }
  }
  async function extract(track: Track) {
    if (busy || downloadingIndex !== null) return;
    error = "";
    downloadingIndex = track.index;
    try {
      if (!outputDir) outputDir = await defaultOutputDirectory();
      const outputPath = await join(outputDir, subtitleOutputName(path, track));
      await invoke("extract_embedded_subtitle", { path, index: track.index, outputPath });
      await refreshDownloaded();
      snackbar.show(`${t("extract.saved")} ${outputPath}`, "success");
    } catch (e) { error = String(e); snackbar.show(error, "error"); }
    finally { downloadingIndex = null; }
  }

</script>
<div class="h-full overflow-y-auto bg-gray-900 p-6 text-gray-100 scrollbar-thin">
  <div class="glass-card p-5" class:ring-2={isDraggingOver} class:ring-teal-400={isDraggingOver}>
    <h3 class="mb-4 text-lg font-semibold text-teal-400 flex items-center gap-2">
      <svg aria-hidden="true" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4h16v12H4zM8 8h3m2 0h3M8 12h8m-4 4v5m-3-3 3 3 3-3" /></svg>
      {t("extract.title")}
    </h3>
    <PathPickerField labelIcon="M15 10l5-3v10l-5-3M4 5h9a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2Z" label={t("flashcards.mediaFile")} value={path} placeholder={t("extract.choose")}
      browseTitle={t("flashcards.browse")} browseLabel={t("flashcards.browse")} onbrowse={chooseMedia}
      disabled={busy || downloadingIndex !== null} onclear={() => { closePreview(); selections = {}; downloaded = []; previewTrack = null; path = ""; tracks = []; tracksScanned = false; error = ""; search = ""; page = 0; }} />
    <div class="mt-3"><PathPickerField label={t("flashcards.outputDir")} labelIcon="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2Z" value={outputDir} placeholder={t("flashcards.outputDir")} browseTitle={t("flashcards.browse")} browseLabel={t("flashcards.browse")} onbrowse={chooseOutput} disabled={busy || downloadingIndex !== null} /></div>
    <div class="mt-3 flex min-h-5 items-center text-sm" role="status" aria-live="polite">
    {#if error || busy}

      {#if error}<span class="text-red-300">{error}</span>
      {:else if busy}<span class="inline-flex" aria-label={t("extract.working")}><LoadingSpinner /></span>{/if}
    {/if}
    </div>
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
      <div class="relative mt-4 grid min-h-[588px] grid-cols-1 lg:grid-cols-2 content-start gap-3" aria-busy={busy}>
        {#if !path && !busy}
          <EmptyStatusLabel message={t("media.noMediaLoaded")} />
        {/if}
        {#each visible as group (group.key)}
          <SubtitleTrackCard {group} {busy} {downloaded} {downloadingIndex} selectedIndex={selections[group.key]} onselect={(index) => selections = { ...selections, [group.key]: index }} ondownload={extract} onpreview={preview} />
        {/each}
        {#if visible.length === 0}
          {#each Array(SUBTITLE_PAGE_SIZE) as _, index (index)}
            <div aria-hidden="true" class="flex min-h-[108px] items-center justify-between rounded-xl border border-white/5 bg-black/10 p-4 {busy ? 'animate-pulse motion-reduce:animate-none' : ''}">
              <div class="space-y-2"><div class="h-3 w-36 rounded bg-white/5"></div><div class="h-2 w-24 rounded bg-white/[0.03]"></div></div>
              <div class="h-10 w-10 rounded-lg border border-white/5"></div>
            </div>
          {/each}
        {/if}
        {#if tracksScanned && tracks.length === 0}
          <div class="pointer-events-none absolute inset-0 flex items-center justify-center p-4" role="status" aria-live="polite">
            <span class="rounded-xl border border-red-400/25 bg-gray-900/95 px-5 py-3 text-center text-sm font-semibold text-red-300 shadow-lg">{t("extract.empty")}</span>
          </div>
        {/if}
      </div>
      {#if tracks.length > 0 && filtered.length === 0}<p class="mt-4 text-sm text-gray-400" role="status">{t("extract.noMatches")}</p>{/if}

  </div>
</div>

{#if previewTrack}
  <SubtitlePreviewModal title={`${t("extract.preview")} · #${previewTrack.index} · ${previewTrack.title || previewTrack.language}`} text={previewText} error={previewError} loading={previewLoading} saving={downloadingIndex !== null} hasPrevious={previewPosition > 0} hasNext={previewPosition + 1 < previewVariants.length} onprevious={() => preview(previewVariants[previewPosition - 1])} onnext={() => preview(previewVariants[previewPosition + 1])} onselect={selectPreview} ondownload={() => previewTrack && extract(previewTrack)} onclose={closePreview} />
{/if}

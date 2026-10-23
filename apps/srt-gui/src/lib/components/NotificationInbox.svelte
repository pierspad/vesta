<script lang="ts">
  import { locale } from "$lib/i18n";
  import { updateCheckerStore as updates } from "$lib/stores/updateCheckerStore.svelte";

  let { hash, releaseUrl, onsettings, onclose }: {
    hash: string;
    releaseUrl: string;
    onsettings: (section: "whisper" | "llm" | "overview") => void;
    onclose: () => void;
  } = $props();
  let t = $derived($locale);
  let pressedBackdrop = false;
  let notesOpen = $state(false);

  function isBackdrop(event: PointerEvent): boolean {
    const dialog = event.currentTarget as HTMLDialogElement;
    const bounds = dialog.getBoundingClientRect();
    return event.target === dialog && (event.clientX < bounds.left || event.clientX > bounds.right
      || event.clientY < bounds.top || event.clientY > bounds.bottom);
  }

  function openDialog(node: HTMLDialogElement) {
    const previous = document.activeElement;
    const scrollPositions: { node: Element; top: number; left: number }[] = [];
    for (let parent = previous?.parentElement; parent; parent = parent.parentElement) {
      scrollPositions.push({node: parent, top: parent.scrollTop, left: parent.scrollLeft});
    }
    const restoreScroll = () => scrollPositions.forEach(({node, top, left}) => {
      if (node.isConnected) { node.scrollTop = top; node.scrollLeft = left; }
    });
    node.showModal();
    // Native dialog focus can scroll the opener's ancestors, including the app shell.
    restoreScroll();
    return { destroy() {
      node.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus({preventScroll: true});
      restoreScroll();
    } };

  }
</script>

<dialog use:openDialog onclose={onclose} aria-labelledby="notification-inbox-title"
  onpointerdown={(event) => { pressedBackdrop = isBackdrop(event); }}
  onpointerup={(event) => {
    if (pressedBackdrop && isBackdrop(event)) onclose();
    pressedBackdrop = false;
  }}
  onpointercancel={() => { pressedBackdrop = false; }}
  class="notification-inbox fixed inset-0 m-auto flex h-auto max-h-[calc(100dvh_-_2rem)] w-[calc(100%_-_2rem)] max-w-4xl flex-col overflow-hidden rounded-2xl border border-white/10 bg-gray-900 p-0 text-gray-200 shadow-2xl">
  <div class="flex shrink-0 items-center justify-between gap-4 border-b border-white/10 bg-gray-900 px-5 py-4">
    <h2 id="notification-inbox-title" class="text-base font-semibold">{t("notifications.title")}</h2>
    <button type="button" onclick={onclose} aria-label={t("common.close")} class="rounded-lg px-2 py-1 text-gray-400 hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-indigo-400">✕</button>
  </div>
  <div class="notification-body">
    {#if updates.updateStatus === "available"}
      <article class="rounded-xl border border-indigo-400/20 bg-indigo-500/5 p-4">
        <h3 class="notice-title"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M12 3v12m-4-4 4 4 4-4M4 16v5h16v-5"/></svg>{t("settings.updatesNewVersionAvailable", { version: updates.latestVersion })}</h3>
        {#if updates.managerHint}<p class="mt-2 text-xs leading-relaxed text-gray-400">{updates.managerHint}</p>{/if}
        {#if (updates.canInstall && !updates.managerHint) || !updates.releaseNotes}
        <div class="mt-3 flex flex-wrap gap-2">
          {#if updates.canInstall && !updates.managerHint}
            <button type="button" onclick={() => updates.installUpdate()} disabled={updates.installing} class="btn-primary px-3 py-2 text-xs disabled:opacity-50">
              {updates.installing ? t("settings.updatesDownloading", { percent: updates.downloadPercent }) : t("settings.updatesInstall", { version: updates.latestVersion })}
            </button>
          {/if}
          {#if !updates.releaseNotes}<button type="button" onclick={() => updates.openRelease()} class="btn-secondary px-3 py-2 text-xs">{t("settings.updatesReleaseDetails")}</button>{/if}
        </div>
        {/if}
      </article>
    {/if}
    {#each ["whisper", "llm"] as section}
      {#if hash.includes(section === "whisper" ? "whisper-model-missing:v1" : "llm-default-unready:v1")}
        <article class="rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <h3 class="notice-title"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 7v6m0 3v1"/></svg>{t(`settings.section.${section}`)}</h3>
          <p class="mt-2 text-xs leading-relaxed text-gray-400">{t(section === "whisper" ? "settings.copy.setupWhisperDesc" : "settings.copy.setupLlmDesc")}</p>
          <button type="button" onclick={() => onsettings(section as "whisper" | "llm")} class="btn-secondary mt-3 px-3 py-2 text-xs">{t("nav.settings")}</button>
        </article>
      {/if}
    {/each}
    {#if updates.releaseNotes || releaseUrl}
      <article class="release-notice">
        {#if updates.releaseNotes}
          <div class="release-details">
            <button type="button" class="release-summary" aria-expanded={notesOpen} aria-controls="notification-release-notes" onclick={() => notesOpen = !notesOpen}>
              <span>{t("settings.updatesReleaseDetails")}</span>
              <span class="release-version">{updates.latestVersion}</span>
              <svg class="release-chevron" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
            </button>
            <div id="notification-release-notes" class="release-notes" hidden={!notesOpen}>
              {#await import("./MarkdownContent.svelte") then module}
                <module.default text={updates.releaseNotes} />
              {/await}
              <a href={releaseUrl} target="_blank" rel="noreferrer" class="btn-secondary mt-3 inline-flex px-3 py-2 text-xs">{t("settings.updatesReleaseDetails")} ↗</a>
            </div>
          </div>
        {:else}
          <a href={releaseUrl} target="_blank" rel="noreferrer" class="btn-secondary inline-flex px-3 py-2 text-xs">{t("settings.updatesReleaseDetails")} ↗</a>
        {/if}
      </article>
    {/if}
  </div>
</dialog>

<style>
  .notification-inbox::backdrop { background: rgb(0 0 0 / 0.65); backdrop-filter: blur(4px); }
  .notification-inbox { block-size: min(720px, calc(100dvh - 2rem)); max-inline-size: 48rem; background: var(--surface-2); color: var(--text); border-color: var(--border); box-shadow: var(--shadow); }
  .notification-inbox > div:first-child { background: var(--surface-2); border-color: var(--border); }
  .notification-body { display: flex; flex-direction: column; gap: .75rem; flex: 1; min-block-size: 0; overflow: hidden; padding: 1.25rem; }
  .notification-body > article:not(.release-notice) { flex-shrink: 0; }
  .release-notice { flex: 1; min-block-size: 0; overflow: hidden; }
  .release-details { display: flex; flex-direction: column; block-size: 100%; min-block-size: 0; }
  .release-summary { flex-shrink: 0; }
  .release-notes { flex: 1; min-block-size: 0; overflow-y: auto; overscroll-behavior: contain; scrollbar-width: thin; scrollbar-color: var(--border) transparent; }
  @media (max-height: 620px) { .notification-body { overflow-y: auto; } .release-notice { flex-shrink: 0; min-block-size: 12rem; } }
  .notification-body > article { border: 1px solid var(--border); background: var(--surface); border-radius: .85rem; }
  .notice-title { display: flex; align-items: center; gap: .65rem; font-size: .9rem; font-weight: 650; color: var(--text); }
  .notice-title svg { flex-shrink: 0; color: var(--accent-text); }
  .release-notice { padding: 0; }
  .release-summary { display: flex; align-items: center; gap: .75rem; padding: 1rem; inline-size: 100%; text-align: start; color: var(--text); cursor: pointer; font-size: .875rem; font-weight: 600; }
  .release-summary:hover { background: var(--surface-3); border-radius: .85rem; }
  .release-summary:focus-visible { outline: 2px solid var(--accent); outline-offset: -3px; border-radius: .85rem; }
  .release-version { margin-inline-start: auto; color: var(--muted); font-size: .75rem; font-weight: 400; }
  .release-chevron { flex-shrink: 0; color: var(--muted); transition: transform .15s ease; }
  .release-summary[aria-expanded="true"] .release-chevron { transform: rotate(180deg); }
  .release-notes { padding: 1rem; border-block-start: 1px solid var(--border); }
</style>

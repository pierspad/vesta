<script lang="ts">
  import { locale } from "$lib/i18n";
  import { updateCheckerStore as updates } from "$lib/stores/updateCheckerStore.svelte";

  let { hash, releaseUrl, onsettings, onclose }: {
    hash: string;
    releaseUrl: string;
    onsettings: (section: "whisper" | "llm" | "itt" | "overview") => void;
    onclose: () => void;
  } = $props();
  let t = $derived($locale);
  let pressedBackdrop = false;
  let notesOpen = $state(false);

  $effect(() => {
    if (updates.appVersionNum) void updates.loadInstalledReleaseNotes?.();
  });

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
  class="notification-inbox">
  <header class="inbox-header">
    <div>
      <h2 id="notification-inbox-title">{t("notifications.title")}</h2>
      <p>{t("notifications.description")}</p>
    </div>
    <button type="button" onclick={onclose} aria-label={t("common.close")} class="btn-icon">✕</button>
  </header>
  <div class="notification-body">
    {#if updates.updateStatus === "available"}
      <article class="update-notice">
        <div class="notice-icon" aria-hidden="true"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M12 3v12m-4-4 4 4 4-4M4 16v5h16v-5"/></svg></div>
        <div class="notice-content">
          <h3 class="notice-title">{t("settings.updatesNewVersionAvailable", { version: updates.latestVersion })}</h3>
          {#if updates.managerHint}<p class="notice-description">{updates.managerHint}</p>{/if}
          {#if !updates.managerHint}
            <div class="notice-actions">
              {#if updates.canInstall}
                <button type="button" onclick={() => updates.installUpdate()} disabled={updates.installing} class="btn-primary btn-compact">
                  {updates.installing ? t("settings.updatesDownloading", { percent: updates.downloadPercent }) : t("settings.updatesInstall", { version: updates.latestVersion })}
                </button>
              {:else}
                <button type="button" onclick={() => updates.openRelease()} class="btn-secondary btn-compact">{t("settings.updatesReleaseDetails")} ↗</button>
              {/if}
            </div>
          {/if}
        </div>
      </article>
    {/if}
    {#if hash}
      <section class="setup-notices" aria-labelledby="notification-setup-title">
        <h3 id="notification-setup-title" class="section-title">{t("notifications.optionalSetup")}</h3>
        {#each [
          { section: "whisper", token: "whisper-model-missing:v1", descKey: "settings.copy.setupWhisperDesc" },
          { section: "llm", token: "llm-default-unready:v1", descKey: "settings.copy.setupLlmDesc" },
          { section: "itt", token: "itt-model-missing:v1", descKey: "settings.copy.setupIttDesc" },
        ] as item}
          {#if hash.includes(item.token)}
            <article class="setup-notice">
              <div class="notice-icon" aria-hidden="true"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><circle cx="12" cy="12" r="9"/><path d="M12 7v6m0 3v1"/></svg></div>
              <div class="notice-content">
                <h4 class="notice-title">{t(`settings.section.${item.section}`)}</h4>
                <p class="notice-description">{t(item.descKey)}</p>
              </div>
              <button type="button" onclick={() => onsettings(item.section as "whisper" | "llm" | "itt")} class="btn-secondary btn-compact setup-action">{t("nav.settings")}</button>
            </article>
          {/if}
        {/each}
      </section>
    {/if}
    {#if updates.installedReleaseNotes || releaseUrl}
      <article class="release-notice">
        {#if updates.installedReleaseNotes}
          <button type="button" class="release-summary" aria-expanded={notesOpen} aria-controls="notification-release-notes" onclick={() => notesOpen = !notesOpen}>
            <span>{t("notifications.installedVersion")}</span>
            <span class="release-version">{updates.appVersionNum}</span>
            <svg class="release-chevron" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
          </button>
          <div id="notification-release-notes" class="release-notes" hidden={!notesOpen}>
            {#if notesOpen}
              {#await import("./MarkdownContent.svelte") then module}
                <module.default text={updates.installedReleaseNotes} />
              {/await}
              {#if releaseUrl}<a href={releaseUrl} target="_blank" rel="noreferrer" class="release-link">{t("settings.updatesReleaseDetails")} ↗</a>{/if}
            {/if}
          </div>
        {:else}
          <a href={releaseUrl} target="_blank" rel="noreferrer" class="release-summary">
            <span>{t("notifications.installedVersion")}</span>
            <span class="release-version">{updates.appVersionNum}</span>
            <span aria-hidden="true">↗</span>
          </a>
        {/if}
      </article>
    {/if}
    {#if updates.updateStatus !== "available" && !hash && !updates.installedReleaseNotes && !releaseUrl}
      <p class="empty-notices">{t("contextMenu.noNotifications")}</p>
    {/if}
  </div>
</dialog>

<style>
  .notification-inbox::backdrop { background: color-mix(in srgb, var(--color-black) 65%, transparent); backdrop-filter: blur(4px); }
  .notification-inbox { position: fixed; inset: 0; margin: auto; padding: 0; display: flex; flex-direction: column; inline-size: min(44rem, calc(100% - 2rem)); max-block-size: calc(100dvh - 2rem); overflow: hidden; border: 1px solid var(--border); border-radius: 1.25rem; background: var(--surface-2); color: var(--text); box-shadow: var(--shadow); }
  .inbox-header { display: flex; align-items: start; justify-content: space-between; gap: 1rem; padding: 1.25rem; border-block-end: 1px solid var(--border); flex-shrink: 0; }
  .inbox-header h2 { font-size: 1rem; font-weight: 650; }
  .inbox-header p { margin-block-start: .3rem; font-size: .8rem; color: var(--muted); }
  .notification-body { display: flex; flex-direction: column; gap: 1.25rem; min-block-size: 0; overflow-y: auto; overscroll-behavior: contain; padding: 1.25rem; }
  .notification-body article { border: 1px solid var(--border); background: var(--surface); border-radius: .85rem; }
  .update-notice, .setup-notice { display: flex; align-items: start; gap: .85rem; padding: 1rem; }
  .notification-body .update-notice { border-color: color-mix(in srgb, var(--accent) 35%, var(--border)); background: color-mix(in srgb, var(--accent) 6%, var(--surface)); }
  .notice-icon { display: flex; flex-shrink: 0; align-items: center; justify-content: center; inline-size: 2.25rem; block-size: 2.25rem; border-radius: .7rem; background: var(--control-bg); color: var(--muted); }
  .update-notice .notice-icon { color: var(--color-indigo-300); }
  .notice-content { flex: 1; min-inline-size: 0; }
  .notice-title { font-size: .875rem; font-weight: 650; line-height: 1.5; overflow-wrap: anywhere; }
  .notice-description { margin-block-start: .3rem; font-size: .8rem; line-height: 1.6; color: var(--muted); }
  .notice-actions { display: flex; flex-wrap: wrap; gap: .5rem; margin-block-start: .85rem; }
  .setup-notices { display: flex; flex-direction: column; gap: .5rem; }
  .section-title { margin-block-end: .1rem; font-size: .75rem; font-weight: 600; color: var(--muted); }
  .setup-action { align-self: center; flex-shrink: 0; }
  .release-notice { flex-shrink: 0; overflow: hidden; }
  .release-summary { display: flex; align-items: center; gap: .75rem; padding: 1rem; inline-size: 100%; text-align: start; color: var(--text); cursor: pointer; font-size: .8rem; font-weight: 600; }
  .release-summary:hover { background: var(--surface-3); }
  .release-summary:focus-visible { outline: 2px solid var(--accent); outline-offset: -3px; }
  .release-version { margin-inline-start: auto; color: var(--muted); font-size: .75rem; font-weight: 400; }
  .release-chevron { flex-shrink: 0; color: var(--muted); transition: transform .15s ease; }
  .release-summary[aria-expanded="true"] .release-chevron { transform: rotate(180deg); }
  .release-notes { padding: 1rem; border-block-start: 1px solid var(--border); }
  .release-link { display: inline-flex; margin-block-start: 1rem; font-size: .8rem; color: var(--color-indigo-300); text-decoration: underline; text-underline-offset: .2em; }
  .empty-notices { padding: 1rem; color: var(--muted); text-align: center; }
  @media (max-width: 540px) {
    .setup-notice { flex-wrap: wrap; }
    .setup-action { margin-inline-start: 3.1rem; }
    .inbox-header, .notification-body { padding: 1rem; }
  }
</style>

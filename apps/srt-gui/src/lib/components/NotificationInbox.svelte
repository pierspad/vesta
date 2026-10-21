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
  class="notification-inbox fixed inset-0 m-auto flex h-[88dvh] max-h-[calc(100dvh_-_2rem)] w-[calc(100%_-_2rem)] max-w-4xl flex-col overflow-hidden rounded-2xl border border-white/10 bg-gray-900 p-0 text-gray-200 shadow-2xl">
  <div class="flex shrink-0 items-center justify-between gap-4 border-b border-white/10 bg-gray-900 px-5 py-4">
    <h2 id="notification-inbox-title" class="text-base font-semibold">{t("notifications.title")}</h2>
    <button type="button" onclick={onclose} aria-label={t("common.close")} class="rounded-lg px-2 py-1 text-gray-400 hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-indigo-400">✕</button>
  </div>
  <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-5">
    {#if updates.updateStatus === "available"}
      <article class="rounded-xl border border-indigo-400/20 bg-indigo-500/5 p-4">
        <h3 class="text-sm font-semibold text-indigo-200">{t("settings.updatesNewVersionAvailable", { version: updates.latestVersion })}</h3>
        {#if updates.managerHint}<p class="mt-2 text-xs leading-relaxed text-gray-400">{updates.managerHint}</p>{/if}
        <div class="mt-3 flex flex-wrap gap-2">
          {#if updates.canInstall && !updates.managerHint}
            <button type="button" onclick={() => updates.installUpdate()} disabled={updates.installing} class="btn-primary px-3 py-2 text-xs disabled:opacity-50">
              {updates.installing ? t("settings.updatesDownloading", { percent: updates.downloadPercent }) : t("settings.updatesInstall", { version: updates.latestVersion })}
            </button>
          {/if}
          <button type="button" onclick={() => updates.openRelease()} class="btn-secondary px-3 py-2 text-xs">{t("settings.updatesReleaseDetails")}</button>
        </div>
      </article>
    {/if}
    {#each ["whisper", "llm"] as section}
      {#if hash.includes(section === "whisper" ? "whisper-model-missing:v1" : "llm-default-unready:v1")}
        <article class="rounded-xl border border-white/10 bg-white/[0.03] p-4">
          <h3 class="text-sm font-semibold text-gray-200">{t(`settings.section.${section}`)}</h3>
          <p class="mt-2 text-xs leading-relaxed text-gray-400">{t(section === "whisper" ? "settings.copy.setupWhisperDesc" : "settings.copy.setupLlmDesc")}</p>
          <button type="button" onclick={() => onsettings(section as "whisper" | "llm")} class="btn-secondary mt-3 px-3 py-2 text-xs">{t("nav.settings")}</button>
        </article>
      {/if}
    {/each}
    <article class="rounded-xl border border-white/10 bg-white/[0.03] p-4">
      <h3 class="text-sm font-semibold">{t("settings.updatesReleaseDetails")}</h3>
      {#if updates.releaseNotes}
        <details class="mt-3 text-sm text-gray-400" open>
          <summary class="cursor-pointer rounded-lg py-1 text-indigo-300">{updates.latestVersion}</summary>
          <div class="mt-4">
            {#await import("./MarkdownContent.svelte") then module}
              <module.default text={updates.releaseNotes} />
            {/await}
          </div>
        </details>
      {/if}
      <a href={releaseUrl} target="_blank" rel="noreferrer" class="btn-secondary mt-3 inline-flex px-3 py-2 text-xs">{t("settings.updatesReleaseDetails")}</a>
    </article>
  </div>
</dialog>

<style>
  .notification-inbox::backdrop { background: rgb(0 0 0 / 0.65); backdrop-filter: blur(4px); }
</style>

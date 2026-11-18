<script lang="ts">
  import { locale } from "$lib/i18n";
  import PathPickerField from "$lib/components/PathPickerField.svelte";

  interface Props {
    installMode: "repo" | "folder";
    urlInput: string;
    inspecting: boolean;
    inspectError: boolean;
    onBrowseFolder: () => void;
    onInspect: (event: MouseEvent) => void;
  }

  let {
    installMode = $bindable(),
    urlInput = $bindable(),
    inspecting,
    inspectError,
    onBrowseFolder,
    onInspect,
  }: Props = $props();

  let t = $derived($locale);
</script>

<div class="rounded-2xl border border-white/10 bg-white/[0.02] p-5 flex flex-col gap-4 shadow-sm">
  <div class="flex items-center justify-between gap-4 flex-wrap">
    <h2 class="text-sm font-semibold text-white flex items-center gap-2">
      <svg aria-hidden="true" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
      </svg>
      {t("plugins.installFromRepo")}
    </h2>

    <div class="inline-flex items-center p-1 rounded-xl bg-black/40 border border-white/10 gap-1 text-xs select-none">
      <button
        type="button"
        onclick={() => installMode = "repo"}
        class="px-3 py-1.5 rounded-lg font-semibold transition-colors cursor-pointer {installMode === 'repo' ? 'bg-white/15 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
      >
        {t("plugins.modeRepo")}
      </button>
      <button
        type="button"
        onclick={() => installMode = "folder"}
        class="px-3 py-1.5 rounded-lg font-semibold transition-colors cursor-pointer {installMode === 'folder' ? 'bg-white/15 text-white shadow-sm' : 'text-gray-400 hover:text-gray-200 hover:bg-white/5'}"
      >
        {t("plugins.modeFolder")}
      </button>
    </div>
  </div>

  {#if installMode === "repo"}
    <div class="h-10 min-h-[40px] flex flex-col sm:flex-row gap-3 items-center">
      <input
        type="text"
        placeholder="https://github.com/owner/repository"
        bind:value={urlInput}
        class="flex-1 w-full h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 outline-none transition-colors"
      />
      <button
        type="button"
        disabled={!urlInput.trim() || inspecting}
        onclick={onInspect}
        class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer shrink-0"
      >
        {#if inspecting}
          <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
          {t("plugins.inspecting")}
        {:else}
          {t("plugins.install")}
        {/if}
      </button>
    </div>
  {:else}
    <div class="h-10 min-h-[40px] flex flex-col sm:flex-row gap-3 items-center">
      <div class="flex-1 w-full h-10">
        <PathPickerField
          value={urlInput}
          placeholder={t("plugins.localFolderPlaceholder")}
          browseTitle={t("plugins.browse")}
          browseLabel={t("flashcards.browse")}
          onbrowse={onBrowseFolder}
          onclear={() => (urlInput = "")}
          containerClass="h-10"
          inputClass="h-10 px-3.5 rounded-xl border border-white/10 bg-white/5 text-sm text-gray-100 placeholder-gray-500 outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          browseButtonClass="btn-secondary h-10 px-3.5 rounded-xl text-xs font-semibold"
          clearButtonClass="h-10 px-3 inline-flex shrink-0 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 hover:border-red-400/60 hover:bg-red-500/20 transition-colors cursor-pointer"
        />
      </div>
      <button
        type="button"
        disabled={!urlInput.trim() || inspecting}
        onclick={onInspect}
        class="btn-primary h-10 px-5 text-sm font-semibold whitespace-nowrap inline-flex items-center justify-center gap-2 disabled:opacity-50 cursor-pointer shrink-0"
      >
        {#if inspecting}
          <svg class="animate-spin h-4 w-4 text-white" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
          {t("plugins.inspecting")}
        {:else}
          {t("plugins.install")}
        {/if}
      </button>
    </div>
  {/if}

  {#if inspectError}
    <p role="alert" class="mt-3 break-words text-sm text-red-300">{t("plugins.inspectFailed")}</p>
  {/if}
</div>

<script lang="ts">
  import { locale } from "$lib/i18n";
  import { getFileName } from "$lib/utils/models";
  import { hasMediaOverrides, type EpisodeMediaOverrides } from "$lib/types/flashcardMediaTypes";

  /** Structural subset of FlashcardsTab.svelte's EpisodeEntry — this table
   * only renders/reports, it never owns the `episodes` array itself (see
   * [[vesta-flashcards-refactor]] memory for why mutations stay in the
   * parent). */
  interface EpisodeRow {
    id: number;
    targetSubsPath: string;
    nativeSubsPath: string;
    mediaPath: string;
    mediaOverrides?: EpisodeMediaOverrides;
  }

  interface Props {
    episodes: EpisodeRow[];
    showSnackbar: (message: string, variant?: "success" | "info" | "warning" | "error") => void;
    onSwapAll: () => void;
    onSwap: (idx: number) => void;
    onEdit: (idx: number) => void;
    onMediaSettings: (idx: number) => void;
    onRemove: (idx: number) => void;
    onContextMenu: (event: MouseEvent, idx: number) => void;
  }
  let { episodes, showSnackbar, onSwapAll, onSwap, onEdit, onMediaSettings, onRemove, onContextMenu }: Props = $props();

  let t = $derived($locale);
</script>

{#if episodes.length === 0}
  <div
    class="p-6 text-center text-gray-500 text-xs border border-dashed border-gray-700 rounded-lg"
  >
    <svg
      class="w-8 h-8 mx-auto mb-2 text-gray-600"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      ><path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="1.5"
        d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"
      /></svg
    >
    {t("flashcards.noFilesAdded")}
  </div>
{:else}
  <div class="border border-gray-700/50 rounded-lg overflow-hidden">
    <div class="overflow-y-auto max-h-[400px]">
      <table class="w-full text-xs table-fixed">
        <thead class="bg-gray-800/80 sticky top-0">
          <tr>
            <th class="p-1.5 text-center text-gray-400 w-10">#</th>
            <th class="p-1.5 text-center text-gray-400"
              >{t("flashcards.targetLangSubs")}</th
            >
            <th class="p-1.5 text-center text-gray-400 w-12">
              <button
                onclick={onSwapAll}
                class="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded-md text-gray-400 transition-colors hover:bg-gray-700/50 hover:text-white"
                title={t("flashcards.swapAllSubs")}
                aria-label={t("flashcards.swapAllSubs")}
              >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
                </svg>
              </button>
            </th>
            <th class="p-1.5 text-center text-gray-400"
              >{t("flashcards.nativeLangSubs")}</th
            >
            <th class="p-1.5 text-center text-gray-400"
              >{t("flashcards.mediaFile")}</th
            >
            <th class="p-1.5 w-28"></th>
          </tr>
        </thead>
        <tbody>
          {#each episodes as ep, idx}
            <tr
              class="border-t border-gray-800 cursor-default {idx % 2 === 0
                ? 'bg-gray-900/30'
                : 'bg-gray-800/20'} hover:bg-gray-700/20"
              oncontextmenu={(e) => onContextMenu(e, idx)}
            >
              <td class="p-1.5 text-center text-gray-500 font-mono">{ep.id}</td>
              <td
                class="p-1.5 cursor-pointer truncate text-emerald-300 transition-colors hover:bg-violet-500/12 hover:text-emerald-100 rounded-md text-center"
                title={ep.targetSubsPath}
                onclick={() => { navigator.clipboard.writeText(ep.targetSubsPath); showSnackbar(t("flashcards.copiedTargetSubs") || "Percorso originale copiato", "success"); }}
              >
                <span class="px-1.5 py-0.5">{getFileName(ep.targetSubsPath)}</span>
              </td>
              <td class="p-1.5 text-center w-12">
                <button
                  onclick={() => onSwap(idx)}
                  class="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded-md text-gray-400 transition-colors hover:bg-gray-700/50 hover:text-white"
                  title={t("flashcards.swapSubs")}
                  aria-label={t("flashcards.swapSubs")}
                >
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
                  </svg>
                </button>
              </td>
              <td
                class="p-1.5 cursor-pointer truncate transition-colors hover:bg-violet-500/12 rounded-md text-center {ep.nativeSubsPath
                  ? 'text-blue-300 hover:text-blue-100'
                  : 'text-gray-600 hover:text-gray-400'}"
                title={ep.nativeSubsPath || "—"}
                onclick={() => { if(ep.nativeSubsPath) { navigator.clipboard.writeText(ep.nativeSubsPath); showSnackbar(t("flashcards.copiedNativeSubs") || "Percorso riferimento copiato", "success"); } }}
              >
                <span class="px-1.5 py-0.5">{ep.nativeSubsPath
                  ? getFileName(ep.nativeSubsPath)
                  : "—"}</span>
              </td>
              <td
                class="p-1.5 cursor-pointer truncate transition-colors hover:bg-violet-500/12 rounded-md text-center {ep.mediaPath ? 'text-purple-300 hover:text-purple-100' : 'text-gray-600 hover:text-gray-400'}"
                title={ep.mediaPath || "—"}
                onclick={() => { if(ep.mediaPath) { navigator.clipboard.writeText(ep.mediaPath); showSnackbar(t("flashcards.copiedMediaPath") || "Percorso media copiato", "success"); } }}
              >
                {#if ep.mediaPath}
                  <span class="group inline-flex max-w-full items-center gap-1.5 text-left px-1.5 py-0.5">
                    {#if hasMediaOverrides(ep.mediaOverrides)}
                      <span
                        class="h-2 w-2 shrink-0 rounded-full bg-violet-400 shadow-[0_0_10px_rgba(167,139,250,0.75)]"
                        title={t("flashcards.hasPerMovieOverrides")}
                        aria-label={t("flashcards.hasPerMovieOverrides")}
                      ></span>
                    {/if}
                    <span class="truncate">
                      {getFileName(ep.mediaPath)}
                    </span>
                  </span>
                {:else}
                  <span class="px-1.5 py-0.5">—</span>
                {/if}
              </td>
              <td class="p-1.5">
                <div class="flex items-center justify-end gap-1">
                <button
                  onclick={(e) => { e.stopPropagation(); onEdit(idx); }}
                  class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-amber-400 transition-colors hover:bg-amber-400/10 hover:text-amber-300"
                  title={t("common.edit")}
                  aria-label={t("common.edit")}
                >
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M11 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2v-5M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"
                    /></svg
                  >
                </button>
                <button
                  onclick={(e) => { e.stopPropagation(); onMediaSettings(idx); }}
                  disabled={!ep.mediaPath}
                  class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-violet-300 transition-colors hover:bg-violet-400/10 hover:text-violet-200 disabled:cursor-not-allowed disabled:opacity-35 disabled:hover:bg-transparent disabled:hover:text-violet-300"
                  title={t("common.settings")}
                  aria-label={t("common.settings")}
                >
                  <svg
                    class="h-4 w-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                    />
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                </button>
                <button
                  onclick={(e) => { e.stopPropagation(); onRemove(idx); }}
                  class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-red-400 transition-colors hover:bg-red-400/10 hover:text-red-300"
                  title={t("common.delete")}
                  aria-label={t("common.delete")}
                >
                  <svg
                    class="w-4 h-4"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    ><path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                    /></svg
                  >
                </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div
      class="bg-gray-800/40 px-2 py-1 text-[10px] text-gray-500 flex items-center justify-between border-t border-gray-700/50"
    >
      <span>{episodes.length} {t("flashcards.seriesEpisodes")}</span>
    </div>
  </div>
{/if}

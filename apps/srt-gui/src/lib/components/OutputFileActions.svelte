<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { locale } from "$lib/i18n";
  import MediaIcon from "./MediaIcon.svelte";
  let { path, packageFile = false }: { path: string; packageFile?: boolean } = $props();
  let t = $derived($locale);
  let error = $state("");
  let opening = $state(false);
  async function open(folder: boolean) {
    opening = true; error = "";
    try { await invoke("open_output_path", { path, folder }); }
    catch (e) { error = String(e); }
    finally { opening = false; }
  }
</script>
<div class="mt-0.5 min-w-0">
  <div class="flex min-w-0 items-center gap-2 text-[11px] text-gray-400">
    <button disabled={opening} onclick={() => open(false)} class="flex min-w-0 items-center gap-1 text-left hover:text-gray-200 hover:underline disabled:opacity-50" title={`${t("common.openOutputFile")}: ${path}`}><MediaIcon kind={packageFile ? "package" : "file"} /><span class="truncate">{path.split(/[\\/]/).pop()}</span></button>
    <button disabled={opening} onclick={() => open(true)} class="shrink-0 rounded p-1 hover:bg-white/10 hover:text-white disabled:opacity-50" title={t("common.openOutputFolder")} aria-label={t("common.openOutputFolder")}><MediaIcon kind="folder" /></button>
  </div>
  {#if error}<p class="mt-1 max-w-md break-words text-xs text-red-300" role="alert">{error}</p>{/if}
</div>

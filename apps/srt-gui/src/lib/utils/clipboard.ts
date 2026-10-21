import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
/** Clipboard denial must not report success or make a cut delete the selection. */
export async function writeClipboard(text: string, successMessage?: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    if (successMessage) snackbar.show(successMessage, "success");
    return true;
  } catch { snackbar.show(t("common.copyFailed"), "error"); return false; }
}
export async function readClipboard(): Promise<string | null> {
  try { return await navigator.clipboard.readText(); }
  catch { snackbar.show(t("common.pasteFailed"), "error"); return null; }
}

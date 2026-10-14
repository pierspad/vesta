import type { SnackbarVariant } from "$lib/stores/snackbarStore.svelte";
import { errorMessage } from "./errorMessage";

/** Keep the operation independent of a particular UI store or native adapter. */
export async function copyText(
  text: string,
  successMessage: string,
  failureMessage: string,
  notify: (message: string, variant: SnackbarVariant) => void,
  write: (text: string) => Promise<void> = text => navigator.clipboard.writeText(text),
): Promise<boolean> {
  try {
    await write(text);
  } catch (error) {
    const detail = errorMessage(error, "");
    notify(detail ? `${failureMessage}: ${detail}` : failureMessage, "error");
    return false;
  }
  notify(successMessage, "success");
  return true;
}

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

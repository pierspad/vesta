import { isTauri } from "@tauri-apps/api/core";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
import { recordSupportEvent } from "$lib/utils/supportLog";

// Desktop clipboard access must not depend on WebView focus/activation rules.
// Never log clipboard contents or backend error messages, which may contain them.
function failed(operation: string, native: boolean, error: unknown) {
  const name = error instanceof Error ? error.name : "ClipboardError";
  recordSupportEvent("clipboard.error", `${operation} (${native ? "native" : "browser"}): ${name}`);
}
/** Clipboard denial must not report success or make a cut delete the selection. */
export async function writeClipboard(text: string, successMessage?: string): Promise<boolean> {
  const native = isTauri();
  try {
    if (native) await writeText(text);
    else await navigator.clipboard.writeText(text);
    if (successMessage) snackbar.show(successMessage, "success");
    return true;
  } catch (error) {
    failed("write", native, error);
    snackbar.show(t("common.copyFailed"), "error");
    return false;
  }
}
export async function readClipboard(): Promise<string | null> {
  const native = isTauri();
  try { return native ? await readText() : await navigator.clipboard.readText(); }
  catch (error) {
    failed("read", native, error);
    snackbar.show(t("common.pasteFailed"), "error");
    return null;
  }
}

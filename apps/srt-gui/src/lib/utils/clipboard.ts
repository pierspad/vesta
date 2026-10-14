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

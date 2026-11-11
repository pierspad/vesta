import { open, save } from "@tauri-apps/plugin-dialog";

import { recordSupportEvent, safeLogValue } from "$lib/utils/supportLog";

let dialogInFlight = false;

async function withDialogLock<T extends string | string[] | null>(kind: string, action: () => Promise<T>): Promise<T | null> {
  if (dialogInFlight) return null;
  dialogInFlight = true;
  try {
    recordSupportEvent("dialog.open", kind);
    const result = await action();
    const count = Array.isArray(result) ? result.length : Number(result !== null);
    recordSupportEvent("dialog.done", `${kind}: ${count ? `${count} selected` : "cancelled"}`);
    return result;
  } catch (error) {
    recordSupportEvent("dialog.error", `${kind}: ${safeLogValue(error)}`);
    throw error;
  } finally {
    dialogInFlight = false;
  }
}

export async function guardedOpen(
  options?: Parameters<typeof open>[0],
): Promise<Awaited<ReturnType<typeof open>> | null> {
  return withDialogLock(options?.directory ? "folder picker" : "file picker", () => open(options));
}

export async function guardedSave(
  options?: Parameters<typeof save>[0],
): Promise<Awaited<ReturnType<typeof save>> | null> {
  return withDialogLock("save dialog", () => save(options));
}

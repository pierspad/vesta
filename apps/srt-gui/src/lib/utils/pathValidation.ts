import { t } from "$lib/i18n";
import { snackbar } from "$lib/stores/snackbarStore.svelte";
export function parentPath(path: string): string {
  const index = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  if (index < 0) return "";
  if (index === 0) return path[0];
  if (index === 2 && /^[a-z]:/i.test(path)) return path.slice(0, 3);
  return path.slice(0, index);
}
/** Uses the same localized notifications for manually edited picker paths. */
export async function validatePickerPath(raw: string, kind: "input" | "output", exists: (path: string) => Promise<boolean>): Promise<string | null> {
  const path = raw.trim();
  if (!path) { snackbar.show(t("common.pathRequired"), "warning"); return null; }
  const required = kind === "input" ? path : parentPath(path);
  try {
    if (required && !await exists(required)) {
      snackbar.show(t(kind === "input" ? "common.fileNotFound" : "common.directoryNotFound", { path: required }), "error");
      return null;
    }
    return path;
  } catch (error) { snackbar.show(String(error), "error"); return null; }
}

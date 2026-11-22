import { invokeCommand } from "$lib/services/tauriClient";

/**
 * Drop-in replacement per `localStorage`, ma persistito da Rust in un file
 * `vesta_config.json` (fuori dalla cache del Webview) invece che nello
 * storage del Webview stesso.
 *
 * Perché: `localStorage` vive nella cache del Webview -- una pulizia della
 * cache o un aggiornamento di sistema può cancellarla silenziosamente,
 * perdendo chiavi API e impostazioni personalizzate. Vedi `commands/config.rs`
 * lato Rust per lo storage vero e proprio (scrittura atomica, tmp+rename).
 *
 * Come funziona: all'avvio `hydrate()` legge l'intero file via
 * `config_load_all` e popola una cache in-memory. Da lì in poi `getItem` è
 * sincrono (legge dalla cache), mentre `setItem`/`removeItem` aggiornano la
 * cache subito e persistono su disco in background (fire-and-forget) --
 * esattamente come si userebbe `localStorage`, quindi i call site esistenti
 * cambiano solo il nome dell'import.
 */

const cache = new Map<string, string>();
let persistenceQueue: Promise<unknown> = Promise.resolve();

function persist(command: string, args: Record<string, unknown>): Promise<unknown> {
  const write = persistenceQueue.then(() => invokeCommand(command, args));
  persistenceQueue = write.catch((error) => console.error("[vestaConfig] persistence failed", error));
  return write;
}

/** Await a durable snapshot before reloading or closing a setup/import flow. */
export async function flush(): Promise<void> {
  await persist("config_replace_all", { values: exportSnapshot() });
}

type ConfigLoadResult = {
  entries: Record<string, string>;
  recovered_backup_path: string | null;
};

let hydrated = false;
let newInstallation = false;
let recoveredBackupPath: string | null = null;

function parseLoadResult(raw: ConfigLoadResult | Record<string, string>): ConfigLoadResult {
  if (
    raw &&
    typeof raw === "object" &&
    "entries" in raw &&
    typeof raw.entries === "object" &&
    raw.entries !== null &&
    "recovered_backup_path" in raw
  ) {
    return {
      entries: raw.entries as Record<string, string>,
      recovered_backup_path:
        typeof raw.recovered_backup_path === "string" ? raw.recovered_backup_path : null,
    };
  }
  return { entries: raw as Record<string, string>, recovered_backup_path: null };
}

/** Deve essere chiamata (e attesa) una sola volta, prima del mount di App. */
export async function hydrate(): Promise<void> {
  if (hydrated) return;
  try {
    const raw = await invokeCommand<ConfigLoadResult | Record<string, string>>("config_load_all");
    const { entries, recovered_backup_path } = parseLoadResult(raw);
    for (const [key, value] of Object.entries(entries)) {
      cache.set(key, value);
    }
    recoveredBackupPath = recovered_backup_path;
    const hasLegacyPreferences = typeof localStorage !== "undefined" && localStorage.length > 0;
    newInstallation = Object.keys(entries).length === 0 && !hasLegacyPreferences && !recoveredBackupPath;
  } catch (e) {
    console.error("[vestaConfig] impossibile leggere vesta_config.json", e);
    throw e;
  }
  await migrateFromLocalStorage();
  await migrateLegacySrtKeys();
  hydrated = true;
}

/** Return and clear any backup path created when recovering from a corrupted config file. */
export function consumeRecoveredBackupPath(): string | null {
  const path = recoveredBackupPath;
  recoveredBackupPath = null;
  return path;
}

/** True only when neither the durable store nor the legacy webview store had preferences at startup. */
export function isNewInstallation(): boolean {
  return newInstallation;
}

// Migrazione una tantum: le installazioni esistenti hanno le impostazioni in
// localStorage. Le importiamo nel config store di Rust al primo avvio dopo
// l'aggiornamento, così nessuno perde chiavi API o note types. Diventa un
// no-op non appena il flag è salvato atomicamente insieme ai valori importati.
const MIGRATION_FLAG_KEY = "__vesta_config_migrated_from_local_storage__";

async function migrateFromLocalStorage(): Promise<void> {
  if (typeof localStorage === "undefined") return;
  if (cache.get(MIGRATION_FLAG_KEY) === "true") return;

  const staged = new Map(cache);
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key || key === MIGRATION_FLAG_KEY) continue;
    const value = localStorage.getItem(key);
    if (value !== null && !staged.has(key)) {
      staged.set(key, value);
    }
  }
  staged.set(MIGRATION_FLAG_KEY, "true");

  try {
    await persist("config_replace_all", { values: Object.fromEntries(staged.entries()) });
    cache.clear();
    for (const [key, value] of staged.entries()) {
      cache.set(key, value);
    }
  } catch (e) {
    console.error("[vestaConfig] migrazione da localStorage fallita", e);
    // Keep migrated preferences available in-memory for this session, but leave
    // the completion flag unset so the next launch retries atomic persistence.
    for (const [key, value] of staged.entries()) {
      if (key !== MIGRATION_FLAG_KEY && !cache.has(key)) {
        cache.set(key, value);
      }
    }
  }
}

export const LEGACY_KEY_RENAMES: Readonly<Record<string, string>> = {
  "srt-tools-api-keys": "vesta-api-keys",
  "srt-tools-custom-models": "vesta-custom-models",
  "srt-default-whisper-model": "vesta-default-whisper-model",
  "srt-tools-ui-language": "vesta-ui-language",
  "srt-tools-shortcut-overrides": "vesta-shortcut-overrides",
};

/** Copy legacy `srt-*` keys to `vesta-*` equivalents when absent and remove old keys. */
export function normalizeLegacyKeys(
  entries: Record<string, string>,
): { values: Record<string, string>; changed: boolean } {
  const values: Record<string, string> = { ...entries };
  let changed = false;
  for (const [oldKey, newKey] of Object.entries(LEGACY_KEY_RENAMES)) {
    if (Object.prototype.hasOwnProperty.call(values, oldKey)) {
      if (!Object.prototype.hasOwnProperty.call(values, newKey)) {
        values[newKey] = values[oldKey];
      }
      delete values[oldKey];
      changed = true;
    }
  }
  return { values, changed };
}

async function migrateLegacySrtKeys(): Promise<void> {
  const { values, changed } = normalizeLegacyKeys(Object.fromEntries(cache.entries()));
  if (!changed) return;

  cache.clear();
  for (const [key, value] of Object.entries(values)) {
    cache.set(key, value);
  }
  try {
    await persist("config_replace_all", { values });
  } catch (e) {
    console.error("[vestaConfig] migrazione chiavi legacy fallita", e);
  }
}

export function getItem(key: string): string | null {
  return cache.get(key) ?? null;
}

/** Lazy imports avoid the config/i18n startup cycle. Background write failures
 * must be visible: an in-memory preference is not proof it reached disk. */
async function reportBackgroundWriteFailure(error: unknown): Promise<void> {
  console.error("[vestaConfig] background write failed", error);
  try {
    const [{ snackbar }, { t }] = await Promise.all([
      import("$lib/stores/snackbarStore.svelte"), import("$lib/i18n"),
    ]);
    snackbar.show(t("common.settingsNotSaved"), "error");
  } catch (notificationError) { console.error("[vestaConfig] could not report write failure", notificationError); }
}

export function setItem(key: string, value: string): void {
  cache.set(key, value);
  void persist("config_set", { key, value }).catch(reportBackgroundWriteFailure);
}

export function removeItem(key: string): void {
  cache.delete(key);
  void persist("config_remove", { key }).catch(reportBackgroundWriteFailure);
}

export function exportSnapshot(): Record<string, string> {
  return Object.fromEntries(cache.entries());
}

export async function replaceAll(values: Record<string, string>): Promise<void> {
  await persist("config_replace_all", { values });
  cache.clear();
  for (const [key, value] of Object.entries(values)) cache.set(key, value);
}

/** Serialize reset after pending preference writes. */
export async function resetInstallation(): Promise<void> {
  await persist("config_reset_installation", {});
  cache.clear();
  recoveredBackupPath = null;
  if (typeof localStorage !== "undefined") localStorage.clear();
  if (typeof sessionStorage !== "undefined") sessionStorage.clear();
}

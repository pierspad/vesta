import { invoke } from "@tauri-apps/api/core";

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
let hydrated = false;

/** Deve essere chiamata (e attesa) una sola volta, prima del mount di App. */
export async function hydrate(): Promise<void> {
  if (hydrated) return;
  try {
    const all = await invoke<Record<string, string>>("config_load_all");
    for (const [key, value] of Object.entries(all)) {
      cache.set(key, value);
    }
  } catch (e) {
    console.error("[vestaConfig] impossibile leggere vesta_config.json", e);
  }
  migrateFromLocalStorage();
  hydrated = true;
}

// Migrazione una tantum: le installazioni esistenti hanno le impostazioni in
// localStorage. Le importiamo nel config store di Rust al primo avvio dopo
// l'aggiornamento, così nessuno perde chiavi API o note types. Diventa un
// no-op non appena il flag è salvato.
const MIGRATION_FLAG_KEY = "__vesta_config_migrated_from_local_storage__";

function migrateFromLocalStorage(): void {
  if (typeof localStorage === "undefined") return;
  if (cache.get(MIGRATION_FLAG_KEY) === "true") return;

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key || key === MIGRATION_FLAG_KEY) continue;
    const value = localStorage.getItem(key);
    if (value !== null && !cache.has(key)) {
      cache.set(key, value);
      void invoke("config_set", { key, value }).catch((e) =>
        console.error(`[vestaConfig] migrazione fallita per "${key}"`, e),
      );
    }
  }

  cache.set(MIGRATION_FLAG_KEY, "true");
  void invoke("config_set", { key: MIGRATION_FLAG_KEY, value: "true" }).catch((e) =>
    console.error("[vestaConfig] impossibile salvare il flag di migrazione", e),
  );
}

export function getItem(key: string): string | null {
  return cache.get(key) ?? null;
}

export function setItem(key: string, value: string): void {
  cache.set(key, value);
  void invoke("config_set", { key, value }).catch((e) =>
    console.error(`[vestaConfig] impossibile salvare "${key}"`, e),
  );
}

export function removeItem(key: string): void {
  cache.delete(key);
  void invoke("config_remove", { key }).catch((e) =>
    console.error(`[vestaConfig] impossibile rimuovere "${key}"`, e),
  );
}

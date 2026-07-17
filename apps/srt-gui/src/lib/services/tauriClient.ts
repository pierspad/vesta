import { invoke } from "@tauri-apps/api/core";

/**
 * Unico punto di importazione di `invoke` da `@tauri-apps/api/core` in tutta
 * la UI.
 *
 * Perché: prima di questo modulo, ogni tab e ogni store importava `invoke`
 * direttamente e chiamava i comandi Rust con stringhe e payload sparsi in
 * ~16 file (100+ call site). Rinominare un comando lato Rust richiedeva un
 * grep su tutta la UI, la stessa chiamata (es. `transcribe_check_file_exists`)
 * era duplicata identica in più tab, e non esisteva un punto solo per
 * aggiungere logging/error-mapping trasversali.
 *
 * Come funziona: i moduli in `src/lib/services/*` sono gli unici a importare
 * questo file (e quindi `@tauri-apps/api/core`); ogni dominio (sync,
 * transcribe, flashcards, ...) espone funzioni tipizzate che i componenti
 * Svelte chiamano senza sapere che sotto c'è Tauri. `invokeCommand` resta un
 * passthrough sottile apposta: non introduce comportamento nuovo, solo un
 * confine architetturale.
 */
export function invokeCommand<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

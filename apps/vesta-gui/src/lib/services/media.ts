import { invokeCommand } from "$lib/services/tauriClient";

/**
 * Comando `flashcard_download_ffmpeg`: nonostante il prefisso storico
 * "flashcard_", scarica il binario ffmpeg condiviso e viene invocato sia da
 * `TranscribeTab.svelte` (per sbloccare i backend di trascrizione) sia da
 * `FlashcardsTab.svelte` (per l'estrazione media). Vive qui, non in
 * `transcribe.ts` o `flashcards.ts`, per non far scegliere a chi lo chiama
 * un dominio "proprietario" arbitrario.
 */
export function downloadFfmpeg(): Promise<void> {
  return invokeCommand<void>("flashcard_download_ffmpeg");
}

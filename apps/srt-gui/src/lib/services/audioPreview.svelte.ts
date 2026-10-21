import { LatestRequest } from "$lib/utils/asyncControl.svelte";
import { invokeCommand as invoke } from "$lib/services/tauriClient";

/** Owns transport, playback and stale-response cleanup for one preview surface. */
export class AudioPreview {
  loading = $state(false);
  playing = $state(false);
  private requests = new LatestRequest();
  private player: HTMLAudioElement | null = null;

  constructor(private onError: (error?: unknown) => void) {}

  async play(args: Record<string, unknown>, isValid: () => boolean = () => true) {
    this.stop();
    const request = this.requests.begin();
    const current = () => request.isCurrent() && isValid();
    this.loading = true;
    try {
      const [port, token] = await invoke<[number, string]>("get_media_server_info");
      if (!current()) return;
      const path = await invoke<string>("flashcard_preview_audio", args);
      if (!current()) return;
      const player = new Audio();
      this.player = player;
      player.onended = () => { if (current()) this.playing = false; };
      player.onerror = () => {
        if (!current()) return;
        this.playing = false;
        this.onError();
      };
      player.src = `http://127.0.0.1:${port}/media?path=${encodeURIComponent(path)}&token=${encodeURIComponent(token)}&_t=${Date.now()}`;
      await player.play();
      if (current()) this.playing = true;
      else player.pause();
    } catch (error) {
      if (current()) {
        this.playing = false;
        this.onError(error);
      }
    } finally {
      if (request.isCurrent()) this.loading = false;
    }
  }

  stop() {
    this.requests.invalidate();
    if (this.player) {
      this.player.onended = null;
      this.player.onerror = null;
      this.player.pause();
      this.player.src = "";
      this.player = null;
    }
    this.playing = false;
    this.loading = false;
  }
}

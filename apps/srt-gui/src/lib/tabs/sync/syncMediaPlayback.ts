import { invokeCommand as invoke } from "$lib/services/tauriClient";
import { errorMessage } from "$lib/utils/errorMessage";

let cachedMediaServerInfo: [number, string] | null = null;

async function getMediaServerInfo(): Promise<[number, string]> {
  if (cachedMediaServerInfo) return cachedMediaServerInfo;
  cachedMediaServerInfo = await invoke<[number, string]>("get_media_server_info");
  return cachedMediaServerInfo;
}

export async function prepareSyncMediaUrl(
  filePath: string,
  {
    setPreparing,
    setAudioError,
    showSnackbar,
    t,
  }: {
    setPreparing: (value: boolean) => void;
    setAudioError: (message: string | null) => void;
    showSnackbar: (message: string, level?: "success" | "info" | "warning" | "error") => void;
    t: (key: string, params?: Record<string, string | number>) => string;
  },
): Promise<string> {
  // Video containers need an audio-only preview even when supported natively:
  // decoding an unused video track can fail in WebKitGTK.
  const needsTranscode = !/\.(mp3|wav|ogg|m4a|aac|opus|flac)$/i.test(filePath);
  if (needsTranscode) {
    setPreparing(true);
    showSnackbar(t("sync.transcodingMedia"), "info");
  }
  try {
    const playbackPath = await invoke<string>("sync_prepare_media_for_playback", {
      path: filePath,
    });
    const [port, token] = await getMediaServerInfo();
    return `http://127.0.0.1:${port}/media?path=${encodeURIComponent(playbackPath)}&token=${token}`;
  } catch (error) {
    setAudioError(`${t("sync.errorLoadingAudio")} ${errorMessage(error)}`);
    throw error;
  } finally {
    if (needsTranscode) {
      setPreparing(false);
    }
  }
}

export async function playSyncMediaElement(
  audioElement: HTMLMediaElement | null,
  {
    source,
    hasAudio,
    audioError,
    isStartingPlayback,
    setStartingPlayback,
    debug,
    showSnackbar,
    t,
  }: {
    source: string;
    hasAudio: boolean;
    audioError: string | null;
    isStartingPlayback: boolean;
    setStartingPlayback: (value: boolean) => void;
    debug: (message: string, payload?: Record<string, unknown>) => void;
    showSnackbar: (message: string, level?: "success" | "info" | "warning" | "error") => void;
    t: (key: string, params?: Record<string, string | number>) => string;
  },
): Promise<boolean> {
  if (!audioElement || !hasAudio) {
    debug(`${source}: play skipped`, { hasAudio, hasElement: !!audioElement, audioError });
    return false;
  }
  if (isStartingPlayback) {
    debug(`${source}: play ignored (already starting)`);
    return false;
  }

  setStartingPlayback(true);
  try {
    debug(`${source}: play requested`, {
      currentTime: audioElement.currentTime,
      paused: audioElement.paused,
      readyState: audioElement.readyState,
    });
    await audioElement.play();
    debug(`${source}: play started`, {
      currentTime: audioElement.currentTime,
      paused: audioElement.paused,
    });
    return true;
  } catch (e) {
    const err = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    const isBenignAbort =
      err.includes("AbortError") ||
      err.toLowerCase().includes("operation was aborted");
    debug(`${source}: play failed`, { error: err });
    if (!isBenignAbort) {
      showSnackbar(`${t("sync.playbackFailed")} — ${err}`, "error");
    }
    return false;
  } finally {
    setStartingPlayback(false);
  }
}

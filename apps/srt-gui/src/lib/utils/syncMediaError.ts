export type Translate = (key: string, params?: Record<string, string | number>) => string;
export function formatSyncMediaError(code: number, message: string, t: Translate): string {
  const codeName = ({ 1: "MEDIA_ERR_ABORTED", 2: "MEDIA_ERR_NETWORK", 3: "MEDIA_ERR_DECODE", 4: "MEDIA_ERR_SRC_NOT_SUPPORTED" } as Record<number, string>)[code] ?? `MEDIA_ERR_${code}`;
  const key = /autoaudiosink|audiosink/i.test(message) ? "sync.mediaError.audioBackend" : code === 3 || code === 4 ? "sync.mediaError.decoder" : "sync.mediaError.generic";
  return t(key, { code: codeName, detail: message });
}

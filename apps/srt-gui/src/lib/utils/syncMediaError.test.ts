import { describe, expect, it } from "vitest";
import { formatSyncMediaError } from "./syncMediaError";
const t = (key: string, params?: Record<string, unknown>) => `${key}:${params?.code}`;
describe("formatSyncMediaError", () => {
  it("classifies decoder and sink failures", () => {
    expect(formatSyncMediaError(3, "decode failed", t)).toBe("sync.mediaError.decoder:MEDIA_ERR_DECODE");
    expect(formatSyncMediaError(3, "autoaudiosink missing", t)).toBe("sync.mediaError.audioBackend:MEDIA_ERR_DECODE");
  });
});

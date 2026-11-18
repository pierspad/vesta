import { initialSyncCheckpoints } from "$lib/workflows/syncAssessment";

export interface SubtitleInfo {
  id: number;
  start_ms: number;
  end_ms: number;
  text: string;
  synced_start_ms: number;
  synced_end_ms: number;
  offset_ms: number;
  is_anchor: boolean;
}

export interface SyncStatus {
  is_loaded: boolean;
  srt_path: string | null;
  video_path: string | null;
  total_subtitles: number;
  anchor_count: number;
  checked_count: number;
  completion_percentage: number;
  average_offset_ms: number;
  suggested_next_id: number | null;
  can_undo: boolean;
  can_redo: boolean;
}

export interface AnchorInfo {
  subtitle_id: number;
  original_time_ms: number;
  corrected_time_ms: number;
  offset_ms: number;
  is_manual: boolean;
}

export const OFFSET_TOLERANCE_MS = 200;

export function formatSyncTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const millis = Math.floor(ms % 1000);
  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
  }
  return `${minutes}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
}

export function formatSyncOffset(ms: number): string {
  const sign = ms >= 0 ? "+" : "";
  return `${sign}${(ms / 1000).toFixed(2)}s`;
}

export function areSyncOffsetsConsistent(manualAnchors: AnchorInfo[]): boolean {
  if (manualAnchors.length < 2) return false;
  const offsets = manualAnchors.map((a) => a.offset_ms);
  return Math.max(...offsets) - Math.min(...offsets) <= OFFSET_TOLERANCE_MS;
}

export function computeNextSyncCheckpoint(
  status: SyncStatus | null,
  wizardHistory: number[],
  manualAnchors: AnchorInfo[],
): number | null {
  if (!status || status.total_subtitles === 0) return null;
  const total = status.total_subtitles;
  for (const cp of initialSyncCheckpoints(total)) {
    if (!wizardHistory.includes(cp)) return cp;
  }
  if (areSyncOffsetsConsistent(manualAnchors)) return null;

  const sortedAnchors = [...manualAnchors].sort((a, b) => a.subtitle_id - b.subtitle_id);
  if (sortedAnchors.length < 2) {
    const mid = Math.round(total / 2);
    return wizardHistory.includes(mid) ? null : mid;
  }

  let maxDiff = 0;
  let bestMid = -1;
  for (let i = 0; i < sortedAnchors.length - 1; i++) {
    const a = sortedAnchors[i];
    const b = sortedAnchors[i + 1];
    const diff = Math.abs(a.offset_ms - b.offset_ms);
    const gap = b.subtitle_id - a.subtitle_id;
    if (diff > OFFSET_TOLERANCE_MS && gap > 1) {
      const mid = Math.round((a.subtitle_id + b.subtitle_id) / 2);
      if (!wizardHistory.includes(mid) && diff > maxDiff) {
        maxDiff = diff;
        bestMid = mid;
      }
    }
  }
  if (bestMid > 0) return bestMid;

  const first = sortedAnchors[0];
  const last = sortedAnchors[sortedAnchors.length - 1];
  if (first.subtitle_id > 2) {
    const mid = Math.round(first.subtitle_id / 2);
    if (!wizardHistory.includes(mid)) return mid;
  }
  if (last.subtitle_id < total - 1) {
    const mid = Math.round((last.subtitle_id + total) / 2);
    if (!wizardHistory.includes(mid)) return mid;
  }
  return null;
}

export const SYNC_MEDIA_EXTENSIONS = [
  "mp3", "wav", "ogg", "flac", "m4a", "aac", "wma", "opus", "m4b",
  "mp4", "mkv", "avi", "webm", "mov", "m4v", "m2ts", "mpeg", "mpg",
];
const SYNC_MEDIA_EXT_SET = new Set(SYNC_MEDIA_EXTENSIONS);

export function isSyncSrtFile(name: string): boolean {
  return name.toLowerCase().endsWith(".srt");
}

export function isSyncMediaFile(name: string): boolean {
  const ext = name.toLowerCase().split(".").pop() || "";
  return SYNC_MEDIA_EXT_SET.has(ext);
}

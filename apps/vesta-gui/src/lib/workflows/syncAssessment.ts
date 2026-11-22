/** Offset agreement at checkpoints is not a statistical match-confidence score. */
export function anchorOffsetSpread(anchors: { offset_ms: number }[]): number | null {
  if (anchors.length < 2) return null;
  let min = Infinity;
  let max = -Infinity;
  for (const anchor of anchors) {
    min = Math.min(min, anchor.offset_ms);
    max = Math.max(max, anchor.offset_ms);
  }
  return max - min;
}

export function initialSyncCheckpoints(total: number): number[] {
  if (total <= 0) return [];
  return [...new Set([1, Math.round(total / 3), Math.round(total * 2 / 3), total]
    .map(id => Math.max(1, Math.min(total, id))))];
}

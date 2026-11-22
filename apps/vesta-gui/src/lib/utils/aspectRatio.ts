/** Resize within export bounds while retaining the ratio captured by the lock. */
export function linkedDimensions(ratio: number, axis: "width" | "height", value: number) {
  if (!Number.isFinite(ratio) || ratio <= 0 || !Number.isFinite(value) || value <= 0) return null;
  const width = Math.min(Math.min(7680, 4320 * ratio), Math.max(Math.max(16, 16 * ratio), axis === "width" ? value : value * ratio));
  return { width: Math.round(width), height: Math.round(width / ratio) };
}

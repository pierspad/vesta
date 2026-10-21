/** Move one item without mutating the persisted source array. */
export function moveTier<T>(tiers: readonly T[], index: number, direction: -1 | 1): T[] | null {
  const target = index + direction;
  if (index < 0 || index >= tiers.length || target < 0 || target >= tiers.length) return null;
  const result = [...tiers];
  [result[index], result[target]] = [result[target], result[index]];
  return result;
}

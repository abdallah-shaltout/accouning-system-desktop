/**
 * `v-model.number` yields `''` when a field is cleared, which silently turns sums into string
 * concatenation. Read raw numeric inputs through these helpers.
 */
export function toNum(v: unknown): number | undefined {
  if (v === '' || v === null || v === undefined) return undefined;
  const n = Number(v);
  return Number.isFinite(n) ? n : undefined;
}

/** Like `toNum`, but empty/invalid counts as 0. */
export function num0(v: unknown): number {
  return toNum(v) ?? 0;
}

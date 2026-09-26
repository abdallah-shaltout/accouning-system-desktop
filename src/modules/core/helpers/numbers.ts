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

/**
 * The ONE rounding rule for money, quantities, costs and rates (plans/pending/21-rust-backend,
 * decision D3): round half **away from zero**, applied to the number's shortest decimal form — the
 * same result `rust_decimal`'s `MidpointAwayFromZero` gives on the exact decimal value, so the mock
 * and the Rust backend agree to the cent:
 *   round2(1.005) = 1.01 · round2(1234.565) = 1234.57 · round2(-0.125) = -0.13 (never -0.12)
 * `Math.round((n + EPSILON) * 100) / 100` gets all three wrong: `n * 100` adds float noise
 * (1.005 → 100.4999…), EPSILON is below one ULP for amounts over ~2, and `Math.round` rounds a
 * negative half toward +∞. Every round2/round4 in the app re-exports these — never add a local copy.
 */
export function roundHalfAwayFromZero(n: number, dp: number): number {
  if (n === 0 || !Number.isFinite(n)) return n === 0 ? 0 : n;
  // 15 significant digits drop binary noise (3.4499999999999997 → 3.45) and keep every real digit.
  const abs = Number(Math.abs(n).toPrecision(15));
  const text = String(abs);
  // Shift the decimal point in text, not by multiplying, so 1.005 becomes exactly 100.5.
  const shifted = text.includes('e') ? abs * 10 ** dp : Number(`${text}e${dp}`);
  const rounded = Number(`${Math.round(shifted)}e-${dp}`);
  return n < 0 && rounded !== 0 ? -rounded : rounded;
}

/** Money (2 decimals) — see `roundHalfAwayFromZero`. */
export function round2(n: number): number {
  return roundHalfAwayFromZero(n, 2);
}

/** Unit cost, quantity and FX rate (4 decimals) — see `roundHalfAwayFromZero`. */
export function round4(n: number): number {
  return roundHalfAwayFromZero(n, 4);
}

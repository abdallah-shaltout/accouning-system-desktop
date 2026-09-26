/**
 * Rounding rule (plans/pending/21-rust-backend decision D3): half away from zero, on the exact
 * decimal value — the same as `rust_decimal`'s `MidpointAwayFromZero`, so the mock and the Rust
 * backend agree to the cent. The cases below are the ones the old `Math.round((n + EPSILON) * 100)
 * / 100` got wrong (negative halves, float noise, amounts where EPSILON is below one ULP).
 */
import { round2 as coreRound2, round4 as coreRound4 } from '../../src/modules/core/helpers/numbers';
import { round2 as mockRound2 } from '../../src/mocks/utils';
import { round4 as mockRound4 } from '../../src/mocks/backend/core';
import { round2 as totalsRound2 } from '../../src/modules/invoices/helpers/totals';
import { round2 as verifyRound2, check, type Result } from './shared';

const ROUND2_CASES: [number, number][] = [
  [1.005, 1.01],
  [1234.565, 1234.57],
  [-0.125, -0.13],
  [-1.005, -1.01],
  [-2.5, -2.5],
  [0.285, 0.29],
  [1.15 * 3, 3.45],
  [0.1 + 0.2, 0.3],
  [-0.004, 0],
  [999_999_999.995, 1_000_000_000],
];

const ROUND4_CASES: [number, number][] = [
  [1.00005, 1.0001],
  [-0.00125, -0.0013],
  [12.34565, 12.3457],
  [10 / 3, 3.3333],
];

export function run(): Result[] {
  const results: Result[] = [];
  for (const [input, expected] of ROUND2_CASES) {
    const actual = coreRound2(input);
    results.push(check(actual === expected && !Object.is(actual, -0), `round2(${input}) = ${expected} (got ${actual})`));
  }
  for (const [input, expected] of ROUND4_CASES) {
    const actual = coreRound4(input);
    results.push(check(actual === expected, `round4(${input}) = ${expected} (got ${actual})`));
  }
  // One rule, not copies: every exported round2/round4 must be the very same function.
  results.push(
    check(
      mockRound2 === coreRound2 && totalsRound2 === coreRound2 && verifyRound2 === coreRound2 && mockRound4 === coreRound4,
      'mocks/utils, invoices/helpers/totals, verify/shared round2 and mocks/backend/core round4 all re-export core/helpers/numbers',
    ),
  );
  return results;
}

/**
 * Shared helpers for `scripts/verify/<area>.ts`. Each area file exports a `run()` that returns a
 * list of `Result`s; `scripts/verify/run.ts` seeds the DB once and calls every area.
 *
 * Invariants come from docs/v2/02-accounting-review.md §4. Some don't apply yet (their posting
 * rules/helpers land in a later phase) — those are reported as `todo(...)`, not failures, per
 * docs/v2/15-action-plan.md Phase 0's "`verify:mocks` v2 harness" item.
 */
import { db } from '../../src/mocks/db';

export interface Result {
  status: 'ok' | 'fail' | 'todo';
  message: string;
}

export function ok(message: string): Result {
  return { status: 'ok', message };
}

export function fail(message: string): Result {
  return { status: 'fail', message };
}

/** An invariant that doesn't apply yet — the phase in parentheses is when it's expected to turn real. */
export function todo(message: string, phase: number): Result {
  return { status: 'todo', message: `${message} (phase ${phase})` };
}

/** `ok(message)` when `cond`, else `fail(message)`. */
export function check(cond: boolean, message: string): Result {
  return cond ? ok(message) : fail(message);
}

export function round2(n: number): number {
  return Math.round(n * 100) / 100;
}

/** Net balance (Σdebit − Σcredit) of an account across every posted journal entry. */
export function glBalance(code: string): number {
  const id = `acc-${code}`;
  return round2(
    db.journalEntries
      .flatMap((e) => e.lines)
      .filter((l) => l.accountId === id)
      .reduce((a, l) => a + l.debit - l.credit, 0),
  );
}

export function closeEnough(a: number, b: number, tolerance = 0.01): boolean {
  return Math.abs(a - b) <= tolerance;
}

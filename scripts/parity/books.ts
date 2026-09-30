/**
 * The books snapshot every case ends with (plan 21 Part 04, B-9), read on both sides through the
 * **same TS services** — so a wrong number anywhere in a case's story shows up here even when the
 * case never asserted it: full-history trial balance, customer and supplier balances, stock per
 * product, and the invariants (mock `runAllInvariants(db)`, Rust `__invariants`) compared by
 * `key`, `passed` and `message`.
 */
import { db } from '../../src/mocks/db';
import { runAllInvariants } from '../../src/mocks/backend/invariants';
import * as reportService from '../../src/modules/reports/services/reportService';
import * as partyService from '../../src/modules/parties/services/partyService';
import { normalize } from './diff';
import type { RecordedError } from './case';
import type { HostInvariant } from './host-protocol';

export type BookPart = { ok: true; value: unknown } | { ok: false; error: RecordedError };

export interface Books {
  trialBalance: BookPart;
  customers: BookPart;
  suppliers: BookPart;
  inventory: BookPart;
}

export type InvariantRow = HostInvariant;

export function toRecordedError(e: unknown): RecordedError {
  if (e && typeof e === 'object' && 'message' in e) {
    const code = (e as { code?: unknown }).code;
    return { code: typeof code === 'string' ? code : 'ERROR', message: String((e as { message: unknown }).message) };
  }
  return { code: 'ERROR', message: String(e) };
}

async function part(fn: () => Promise<unknown>): Promise<BookPart> {
  try {
    return { ok: true, value: normalize(await fn()) };
  } catch (e) {
    return { ok: false, error: toRecordedError(e) };
  }
}

/** Reads the books through the services (whichever backend `usesRust()` currently points at). */
export async function readBooks(): Promise<Books> {
  return {
    trialBalance: await part(() => reportService.getTrialBalance({})),
    customers: await part(() => partyService.getCustomers()),
    suppliers: await part(() => partyService.getSuppliers()),
    inventory: await part(() => reportService.getInventoryReport()),
  };
}

/** The mock's invariants, reduced to what is compared with Rust.
 *
 * `runAllInvariants` builds its result as one flat array-spread expression with no per-check
 * try/catch (it's the one shared copy `verify:mocks`/the debug-mode watcher also use, and per
 * CLAUDE.md's accounting-safety rules it stays that single copy — never forked here). A base whose
 * chart of accounts is missing a system role `accountFor()` needs (`checkArApControl`/
 * `checkInventoryGl` both call it for `receivable`/`payable`/`inventory`) throws instead of
 * returning a `passed: false` row — `src-tauri/tests/fixtures/mock-snapshot-edge.json` is exactly
 * this case (only `cash`/`4010`/vat-output roles). Per the harness contract ("an invariant that
 * throws is recorded as a failed invariant result, not a crash" — the fixture itself is off-limits,
 * `src/mocks/backend/invariants.ts` is off-limits too, this file is the harness's own seam), that
 * throw is caught here and turned into one synthetic failing row instead of aborting `invariantsNow()`
 * (which every `s.step`/`s.expectError` call runs after, and which the runner also calls before and
 * after the whole case for the baseline/final books) — so a base like `edge` can still run every
 * other check the harness does (steps, books) instead of crashing the whole pass before any case
 * code executes. */
export function mockInvariants(): InvariantRow[] {
  try {
    return runAllInvariants(db).map(({ key, passed, message }) => ({ key, passed, message }));
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return [{ key: 'invariants-crashed', passed: false, message }];
  }
}

/** `key: message` for every failing invariant — the form baseline subtraction works on. */
export function failing(rows: InvariantRow[]): string[] {
  return rows.filter((r) => !r.passed).map((r) => `${r.key}: ${r.message}`);
}

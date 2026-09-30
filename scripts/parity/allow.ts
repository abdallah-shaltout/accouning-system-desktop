/**
 * The parity allowlist (plan 21 Part 04, B-8, decision P4-6). Nothing is waved through silently:
 * every entry must cite a written decision or quirk id from a domain file, or the run fails.
 *
 * `GLOBAL_ALLOW` applies to every case. It is seeded by phase A's A-6 (each reviewed
 * `// contract-ok:` exception that is a real representation difference is copied here, with the same
 * reason), so the runtime diff tolerates exactly what the type check tolerates, and nothing more.
 * A case adds its own entries through `defineCase({ allow: [...] })`.
 */
import type { AllowEntry } from './case';
import { compilePattern, type Diff } from './diff';

/**
 * Every-case entries (phase A, A-6 review of the 19 `// contract-ok:` type-check exceptions,
 * `grep -rn "contract-ok" src/modules`). Only 1 of the 19 has a corresponding runtime JSON path that
 * a recorded parity step could actually show a spurious diff on — every other one is a *static*
 * type-check artifact (an interface-vs-type-literal `Simplify` quirk, a request-only field the Rust
 * side never reads back, a TS-only import like `AppRoute`, or a `null→undefined` switch-line mapping
 * that already makes both sides agree before the value is ever recorded) with no matching path in the
 * diff tree at all, so seeding one for it would either match nothing or require an overly broad `**`
 * pattern that could mask a real future bug — exactly what P4-6 ("nothing gets waved through
 * silently") rules out. The full per-exception review:
 *
 * - `core/types/contract.check.ts` `_InsightDto` (icon/roles), `_AttachmentMeta`
 *   (AttachmentRecord's blob/thumbnail is compared as its own type, `_AttachmentMeta`, not here) —
 *   both mapped 1:1 at the service seam (`insightEngine.ts`'s `toInsight`,
 *   `attachmentService.ts`'s `recordFromDto`) before the value is ever recorded; no diff.
 * - `diagnostics/types/contract.check.ts` `_AuditFilter` (Id-newtype-vs-string identity),
 *   `_PostingTrace` (Simplify wrapper for the Omit<>&{} intersection) — type-identity artifacts, not
 *   value differences.
 * - `diagnostics/services/accountingDebugService.ts` `getPostingTrace`/`getJournalEntryRaw`,
 *   `parties/services/partyService.ts` `getLinkedNetBalance` (+ its `parties/types/contract.check.ts`
 *   companion) — each `?? undefined` at the switch line converts Rust's `null` to `undefined` before
 *   the case ever sees it, and the mock never returns `null` either, so both sides record the same
 *   `T | undefined`; the mapping is what MAKES parity hold, not a tolerated gap.
 * - `invoices/types/contract.check.ts` `_OpenShiftInput` (terminalId request-only, server-owned,
 *   never read back), `_ShiftRow` (Simplify over an inferred intersection).
 * - `payments/types/contract.check.ts` `_PaymentRow`, `reports/types/contract.check.ts`
 *   `_AccountLedger` (AppRoute imported not generated — same TS type both sides),
 *   `settings/types/contract.check.ts` `_StoreSettings` (Partial request shape),
 *   `setup/types/contract.check.ts` `_LegacySnapshotSummary`/`_OnboardingProgress` (interface vs
 *   type-literal), `templates/types/contract.check.ts` `_PdfTemplate` (opaque JSON, same TS type
 *   both sides), `vouchers/types/contract.check.ts` `_Voucher` (tagged union checked per variant,
 *   same effective shape) — all `Simplify`/import/request-shape type-identity artifacts.
 *
 * `diagnostics.getPostingTrace`'s `lines[]` element (21 §16 D-5, dev-only diagnostics) is the one
 * exception with a real, if narrow, runtime possibility: the mock's declared element type is the
 * full persisted `JournalLine` shape while Rust's `TraceLine` structurally never carries an `id` (a
 * trace is recorded before the journal entry/lines are persisted on both sides, so the mock's `id`
 * is expected to be absent/undefined at runtime too — but this is dev-only debugger data assembled
 * ad hoc rather than a typed DTO round-trip, so it is allowlisted defensively instead of assumed.
 */
/*
 * Not here on purpose: `createdAt` present only on Rust for an imported row (00-import D-8) is
 * handled row-precisely in `diff.ts` (P4-13 d — only base-snapshot rows that had no createdAt), and
 * `null` vs absent is a diff normalization rule there (P4-13 a), not an allowlist entry.
 */
export const GLOBAL_ALLOW: AllowEntry[] = [
  {
    path: 'steps.*.value.lines[].id',
    reason:
      "16 D-5 / 21.04 A-6: diagnostics.getPostingTrace's lines[] is Rust's TraceLine (shared::ledger::trace), which never carries a persisted line id (the trace is recorded before journal_lines exist) — the mock's declared JournalLine element type allows one, so a mock-side lines[].id (present or absent) is not compared against Rust's (always-absent) id",
  },
];

/**
 * A decision/quirk id: `A-D1`, `Q-I10`, `R-7`, `D-8`, `P4-6`, `G-25`, `H-1`, `ACC-0003` (letters,
 * optional digits, a hyphen, then an optional letter and digits), `Q6`, or a domain-file item
 * reference `12-accounting #10` / `12 #10` / `13b #3`. A `§` section number alone is not an id.
 */
const DECISION_ID_RES: RegExp[] = [
  /\b[A-Z][A-Z0-9]{0,5}-[A-Z]{0,3}\d+[a-z]?\b/,
  /\bQ\d+\b/,
  /\b\d{2}[a-z]?(?:-[a-z][a-z-]*)?\s*#\d+\b/,
];

export function citesDecision(reason: string): boolean {
  return DECISION_ID_RES.some((r) => r.test(reason));
}

/** Returns one error line per invalid entry (empty when all are valid). */
export function validateAllow(entries: AllowEntry[], where: string): string[] {
  const errors: string[] = [];
  for (const e of entries) {
    if (!e.path?.trim()) errors.push(`${where}: allow entry without a path`);
    else if (!e.reason?.trim() || !citesDecision(e.reason)) {
      errors.push(`${where}: allow entry "${e.path}" — reason "${e.reason ?? ''}" does not cite a decision or quirk id (e.g. A-D1, Q6, R-7, 12-accounting #10, D-8)`);
    }
  }
  return errors;
}

/** Marks every diff an entry covers (the entry's node or anything below it) with that entry's reason. */
export function applyAllow(diffs: Diff[], entries: AllowEntry[]): void {
  const compiled = entries.map((e) => ({
    re: compilePattern(e.path, !e.emptyArrayOnly && !e.dayOfInstantOnly),
    reason: e.reason,
    emptyArrayOnly: !!e.emptyArrayOnly,
    dayOfInstantOnly: !!e.dayOfInstantOnly,
  }));
  for (const d of diffs) {
    if (d.allowedBy) continue;
    const hit = compiled.find(
      (c) => c.re.test(d.path) && (!c.emptyArrayOnly || isEmptyArrayVsAbsent(d)) && (!c.dayOfInstantOnly || isInstantVsItsDay(d)),
    );
    if (hit) d.allowedBy = hit.reason;
  }
}

/** Mock an ISO instant, Rust a `YYYY-MM-DD` day it falls on in some UTC−14…UTC+14 zone (`AllowEntry.dayOfInstantOnly`). */
function isInstantVsItsDay(d: Diff): boolean {
  if (d.kind !== 'value' || typeof d.mock !== 'string' || typeof d.rust !== 'string') return false;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(d.rust) || !/T\d{2}:\d{2}/.test(d.mock)) return false;
  const instant = Date.parse(d.mock);
  const dayStart = Date.parse(`${d.rust}T00:00:00.000Z`);
  if (Number.isNaN(instant) || Number.isNaN(dayStart)) return false;
  const H = 3_600_000;
  return instant >= dayStart - 14 * H && instant < dayStart + 24 * H + 14 * H;
}

/** One side absent, the other `[]` (`AllowEntry.emptyArrayOnly`). */
function isEmptyArrayVsAbsent(d: Diff): boolean {
  const empty = (v: unknown) => Array.isArray(v) && v.length === 0;
  return (d.kind === 'extra' && d.mock === undefined && empty(d.rust)) || (d.kind === 'missing' && d.rust === undefined && empty(d.mock));
}

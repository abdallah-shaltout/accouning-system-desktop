/**
 * Posting trace (18.F1, docs/v2/02-accounting-review.md). Built at the mock backend's one choke
 * point for every ledger posting — `resolvePosting`/`postJournal` in `core.ts` — so any wrong
 * number can be walked back to the exact step that produced it, without changing the posting math
 * itself. This module only OBSERVES what `core.ts` already computed; it never decides an account,
 * amount or rounding.
 *
 * Storage: kept in an in-memory ring buffer here (module-level state, not `db`), and — only when
 * accounting debug mode is on (`equal.debug` includes `accounting`, same switch `logService` uses
 * for the `accounting` channel) — also kept in a `Map` keyed by journal entry id so the inspector
 * (18.F3) can look one up beside its entry. Neither structure is part of `MockDb`, so neither ever
 * reaches `collectBackupData()` (`backupArchive.ts` clones `db` only) — traces must never end up
 * in a backup.
 */
import type { JournalLine } from '@/modules/accounting/types';
import { log } from '@/modules/diagnostics/services/logService';
import type { PostingLine } from './core';

/** One step in a posting's derivation. `kind` groups steps for the inspector's tabs; `detail` is
 * free-form (numbers, chosen ids, "why") so this doesn't need a new shape per accounting concept. */
export interface PostingTraceStep {
  kind: 'lineDiscount' | 'invoiceDiscount' | 'vat' | 'accountResolution' | 'cost' | 'fx' | 'note';
  label: string;
  detail: Record<string, unknown>;
}

export interface PostingTrace {
  docType: string;
  docId: string;
  correlationId?: string;
  steps: PostingTraceStep[];
  lines: JournalLine[];
  totals: { dr: number; cr: number };
  balanced: boolean;
  at: string;
}

const RING_SIZE = 500;
const ring: PostingTrace[] = [];

/** Debug-mode only: trace kept beside its journal entry (id -> trace), so the F3 inspector can
 * pull it up without scanning the ring. Cleared along with everything else on a hard reset — there
 * is no eviction policy beyond the ring's own cap feeding this map's size indirectly. */
const byEntryId = new Map<string, PostingTrace>();

function debugAccountingEnabled(): boolean {
  try {
    const raw = localStorage.getItem('equal.debug');
    if (!raw) return false;
    return raw.split(',').map((s) => s.trim()).some((p) => p === 'accounting' || p === '*');
  } catch {
    return false;
  }
}

/** Per-line annotations a caller may have attached to its `PostingLine`s before calling
 * `resolvePosting`/`postJournal` (F1: "extend `PostingLine` with optional step-annotation fields
 * callers can attach, defaulting gracefully when absent"). Every field is optional — a caller that
 * knows nothing about tracing still posts exactly as before. */
function stepsFromAnnotations(pairs: { input: PostingLine; resolved: JournalLine }[]): PostingTraceStep[] {
  const steps: PostingTraceStep[] = [];
  pairs.forEach(({ input: l, resolved: r }) => {
    if (l.trace?.lineDiscount) {
      steps.push({ kind: 'lineDiscount', label: `خصم على السطر${r ? ` (${r.description ?? ''})` : ''}`, detail: l.trace.lineDiscount });
    }
    if (l.trace?.invoiceDiscount) {
      steps.push({ kind: 'invoiceDiscount', label: 'توزيع خصم الفاتورة', detail: l.trace.invoiceDiscount });
    }
    if (l.trace?.vat) {
      steps.push({ kind: 'vat', label: 'ضريبة القيمة المضافة', detail: l.trace.vat });
    }
    if (l.trace?.cost) {
      steps.push({ kind: 'cost', label: 'تكلفة (متوسط مرجح)', detail: l.trace.cost });
    }
    if (l.trace?.fx) {
      steps.push({ kind: 'fx', label: 'تحويل عملة', detail: l.trace.fx });
    }
    if (l.trace?.note) {
      steps.push({ kind: 'note', label: l.trace.note, detail: {} });
    }
    // Account resolution: always knowable at this choke point, whether the caller annotated or
    // not — `role` -> the account actually chosen; `accountId` -> a direct pick (manual journal,
    // user-selected account), which has no "candidates tried" to report.
    if (r) {
      steps.push({
        kind: 'accountResolution',
        label: 'اختيار الحساب',
        detail: l.role
          ? { role: l.role, chosenAccountId: r.accountId, why: `role="${l.role}"${l.branchId ? ` branchId="${l.branchId}"` : ''}${l.currency ? ` currency="${l.currency}"` : ''}` }
          : { chosenAccountId: r.accountId, why: 'accountId مباشر من المستدعي (قيد يدوي أو حساب مختار)' },
      });
    }
  });
  return steps;
}

/**
 * Builds and records a trace for one posting. Called from `resolvePosting`/`postJournal` in
 * `core.ts` right after the lines resolve — never changes anything it's given, only reads it.
 * `docId` is the journal entry id once posted, or a temporary `pending:<n>` tag when called from
 * `resolvePosting` alone (draft save, or a caller that never reaches `postJournal`).
 */
export function recordPostingTrace(opts: {
  docType: string;
  docId: string;
  correlationId?: string;
  pairs: { input: PostingLine; resolved: JournalLine }[];
  totalDebit: number;
  totalCredit: number;
}): PostingTrace {
  const steps = stepsFromAnnotations(opts.pairs);
  const trace: PostingTrace = {
    docType: opts.docType,
    docId: opts.docId,
    correlationId: opts.correlationId,
    steps,
    lines: opts.pairs.map((p) => p.resolved),
    totals: { dr: opts.totalDebit, cr: opts.totalCredit },
    balanced: Math.abs(opts.totalDebit - opts.totalCredit) <= 0.001,
    at: new Date().toISOString(),
  };

  ring.push(trace);
  if (ring.length > RING_SIZE) ring.splice(0, ring.length - RING_SIZE);

  if (debugAccountingEnabled()) {
    byEntryId.set(opts.docId, trace);
  }

  log.accounting('backend.postJournal', `posting trace: ${opts.docType} ${opts.docId}`, {
    docType: trace.docType,
    docId: trace.docId,
    correlationId: trace.correlationId,
    stepCount: trace.steps.length,
    totals: trace.totals,
    balanced: trace.balanced,
  });

  return trace;
}

/** Last N traces, most recent first — the in-memory ring for the F3 inspector's document picker. */
export function recentPostingTraces(limit = RING_SIZE): PostingTrace[] {
  return ring.slice(-limit).reverse();
}

/** The trace stored beside a specific journal entry (debug mode only — empty otherwise). */
export function postingTraceFor(entryId: string): PostingTrace | undefined {
  return byEntryId.get(entryId);
}

export function clearPostingTraces(): void {
  ring.length = 0;
  byEntryId.clear();
}

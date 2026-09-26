/**
 * Read-side for the accounting debugger (18.F3) — powers `/dev/diagnostics`'s "المحاسبة" tab
 * (`DevAccountingTab.vue`): document picker, posting trace, journal lines, account resolution,
 * balances before/after, invariants, and the subledger-vs-GL drift report. Dev-only, read-only:
 * this file never mutates anything, it only reads `db` and the diagnostics ring buffers — kept
 * seam-safe like every other service (pages never import `src/mocks/*` directly).
 */
import {
  clone,
  customerBalance,
  db,
  delay,
  round2,
  runAllInvariants,
  recentPostingTraces,
  postingTraceFor,
  supplierBalance,
  unallocatedCreditFor,
  type InvariantResult,
  type PostingTrace,
} from '@/mocks';
import { wrap } from './defineService';

export interface AccountingDocSummary {
  id: string;
  number: string;
  date: string;
  description: string;
  type: string;
  totalDebit: number;
  totalCredit: number;
  sourceKind?: string;
  hasTrace: boolean;
}

/** Most recent posted entries (newest first) for the inspector's document picker, joined with
 * whether a posting trace is available for it (debug mode on + traced since it was posted). */
export const listRecentDocuments = wrap('diagnostics.listRecentDocuments', async function listRecentDocuments(limit = 100): Promise<AccountingDocSummary[]> {
  await delay(60);
  const traced = new Set(recentPostingTraces().map((t) => t.docId));
  return [...db.journalEntries]
    .sort((a, b) => b.date.localeCompare(a.date) || b.number.localeCompare(a.number))
    .slice(0, limit)
    .map((e) => ({
      id: e.id,
      number: e.number,
      date: e.date,
      description: e.description,
      type: e.type,
      totalDebit: e.totalDebit,
      totalCredit: e.totalCredit,
      sourceKind: e.sourceRef?.kind,
      hasTrace: traced.has(e.id),
    }));
});

/** The posting trace for one entry, from the in-memory ring (works even with debug mode off — the
 * ring always records; only the beside-the-entry `byEntryId` store needs debug mode). Falls back
 * to `undefined` when the entry predates this session (ring is in-memory only, cleared on reload)
 * or debug mode was off the whole time and the ring already rotated past it. */
export const getPostingTrace = wrap('diagnostics.getPostingTrace', async function getPostingTrace(entryId: string): Promise<PostingTrace | undefined> {
  await delay(30);
  return postingTraceFor(entryId) ?? recentPostingTraces().find((t) => t.docId === entryId);
});

export const getJournalEntryRaw = wrap('diagnostics.getJournalEntryRaw', async function getJournalEntryRaw(id: string) {
  await delay(30);
  const entry = db.journalEntries.find((e) => e.id === id) ?? db.journalDrafts.find((e) => e.id === id);
  return entry ? clone(entry) : undefined;
});

/** Every account touched by an entry's lines, with its balance immediately before and immediately
 * after this entry — "before" = every other posted entry dated on-or-before this one, excluding
 * this entry itself; "after" = including it. Same-day entries are ordered by number so the
 * before/after pair is deterministic. */
export const getBalancesAround = wrap('diagnostics.getBalancesAround', async function getBalancesAround(entryId: string) {
  await delay(60);
  const entry = db.journalEntries.find((e) => e.id === entryId);
  if (!entry) return [];
  const accountIds = [...new Set(entry.lines.map((l) => l.accountId))];
  const priorOrSame = db.journalEntries.filter(
    (e) => e.date < entry.date || (e.date === entry.date && e.number <= entry.number),
  );
  const before = db.journalEntries.filter(
    (e) => e.date < entry.date || (e.date === entry.date && e.number < entry.number),
  );
  return accountIds.map((accountId) => {
    const account = db.accounts.find((a) => a.id === accountId);
    const sum = (entries: typeof priorOrSame) =>
      round2(entries.flatMap((e) => e.lines).filter((l) => l.accountId === accountId).reduce((a, l) => a + l.debit - l.credit, 0));
    return {
      accountId,
      accountName: account?.name ?? accountId,
      accountCode: account?.code,
      before: sum(before),
      after: sum(priorOrSame),
    };
  });
});

export const getInvariantResults = wrap('diagnostics.getInvariantResults', async function getInvariantResults(): Promise<InvariantResult[]> {
  await delay(80);
  return runAllInvariants(db);
});

export interface DriftRow {
  kind: 'customer' | 'supplier' | 'product';
  id: string;
  label: string;
  subledger: number;
  gl: number;
  diff: number;
  firstDivergingDocId?: string;
  firstDivergingDocNumber?: string;
}

/** Subledger vs GL drift per party and per product (18.F3 "drift report"). Re-derives the same
 * running-balance walk `invariants.ts` uses for its pass/fail check, but keeps every row (not just
 * the failing ones) and records the first posted entry where the running subledger total first
 * stopped matching the running GL total for that account — the same "which document" question the
 * inspector exists to answer. Read-only; duplicates a small amount of the invariant math on
 * purpose so this dev-only report never has to import backend internals beyond what `@/mocks`
 * already re-exports. */
export const getDriftReport = wrap('diagnostics.getDriftReport', async function getDriftReport(): Promise<DriftRow[]> {
  await delay(120);
  const rows: DriftRow[] = [];

  const receivableAccountIds = new Set(db.accounts.filter((a) => a.systemRole === 'receivable').map((a) => a.id));
  const payableAccountIds = new Set(db.accounts.filter((a) => a.systemRole === 'payable').map((a) => a.id));
  const inventoryAccountIds = new Set(db.accounts.filter((a) => a.systemRole === 'inventory').map((a) => a.id));

  const sortedEntries = [...db.journalEntries].sort((a, b) => a.date.localeCompare(b.date) || a.number.localeCompare(b.number));

  /** Walks entries chronologically, tracking running GL for this party's accountIds; the first
   * entry after which running GL stops matching the (already-final) subledger total is reported —
   * exact once the party has no more activity after the drift starts, an approximation otherwise
   * (good enough for "which document to look at first", which is what the inspector needs). */
  function firstDivergingEntry(accountIds: Set<string>, partyId: string, subledgerTotal: number, sign: 1 | -1): { id?: string; number?: string } {
    let running = 0;
    for (const e of sortedEntries) {
      const delta = e.lines.filter((l) => accountIds.has(l.accountId) && l.partyId === partyId).reduce((a, l) => a + l.debit - l.credit, 0) * sign;
      if (delta === 0) continue;
      running = round2(running + delta);
      if (Math.abs(running - subledgerTotal) > 0.01) return { id: e.id, number: e.number };
    }
    return {};
  }

  // §4.6: subledger = Σ outstanding (non-draft invoices/POs, net of refunds/returns) − unallocated
  // credit; gl = customerBalance()/supplierBalance() (the control account itself). Same formula
  // `checkPartyAllocation` in invariants.ts uses, kept here only for display (every row, not just
  // failing ones) — this file never decides pass/fail, that stays in the shared invariant.
  const arReducedByRefunds = (invoiceId: string) => db.refunds.filter((r) => r.invoiceId === invoiceId).reduce((a, r) => a + r.settledToReceivable, 0);
  for (const c of db.customers) {
    const outstanding = round2(
      db.invoices.filter((i) => i.customerId === c.id && i.status !== 'DRAFT').reduce((a, i) => a + (i.grandTotal - arReducedByRefunds(i.id) - i.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('customer', c.id);
    const subledger = round2(outstanding - credit);
    const gl = customerBalance(c.id);
    const diff = round2(subledger - gl);
    if (Math.abs(diff) > 0.01) {
      const first = firstDivergingEntry(receivableAccountIds, c.id, gl, 1);
      rows.push({ kind: 'customer', id: c.id, label: c.name, subledger, gl, diff, firstDivergingDocId: first.id, firstDivergingDocNumber: first.number });
    }
  }

  const apReducedByReturns = (poId: string) =>
    db.purchaseReturns.filter((r) => r.purchaseOrderId === poId).reduce((a, r) => a + (r.refundMethod === 'credit' ? r.grandTotal : r.settledToPayable), 0);
  for (const s of db.suppliers) {
    const outstanding = round2(
      db.purchaseOrders.filter((p) => p.supplierId === s.id && p.status === 'RECEIVED').reduce((a, p) => a + (p.grandTotal - apReducedByReturns(p.id) - p.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('supplier', s.id);
    const subledger = round2(outstanding - credit);
    const gl = supplierBalance(s.id);
    const diff = round2(subledger - gl);
    if (Math.abs(diff) > 0.01) {
      const first = firstDivergingEntry(payableAccountIds, s.id, -gl, -1);
      rows.push({ kind: 'supplier', id: s.id, label: s.name, subledger, gl, diff, firstDivergingDocId: first.id, firstDivergingDocNumber: first.number });
    }
  }

  // Products: no per-product journal dimension exists (inventory postings aren't tagged with
  // partyId/productId on the journal line), so drift here can only compare the product's own
  // stockValue against its share of the inventory GL total — flagged only when the aggregate
  // invariant (checkInventoryGl) is already failing, since a per-product GL breakdown isn't
  // derivable from journal lines alone in this schema.
  const invGl = round2(
    db.journalEntries.flatMap((e) => e.lines).filter((l) => inventoryAccountIds.has(l.accountId)).reduce((a, l) => a + l.debit - l.credit, 0),
  );
  const invSum = round2(db.products.reduce((a, p) => a + (p.type === 'product' ? p.stockValue : 0), 0));
  if (Math.abs(invGl - invSum) > 0.01) {
    for (const p of db.products) {
      if (p.type !== 'product') continue;
      rows.push({ kind: 'product', id: p.id, label: p.name, subledger: p.stockValue, gl: invGl, diff: round2(invSum - invGl) });
    }
  }

  return rows;
});

/** "اشرح هذا الرقم" (18.F3): every posted journal line touching `accountId`, optionally narrowed to
 * a party, grouped by its source document — each row links back to its own trace via `docId`. */
export interface ExplainLine {
  docId: string;
  docNumber: string;
  docDate: string;
  sourceKind?: string;
  description?: string;
  debit: number;
  credit: number;
}

export const explainAccountBalance = wrap(
  'diagnostics.explainAccountBalance',
  async function explainAccountBalance(accountId: string, partyId?: string): Promise<ExplainLine[]> {
    await delay(80);
    const out: ExplainLine[] = [];
    for (const e of db.journalEntries) {
      for (const l of e.lines) {
        if (l.accountId !== accountId) continue;
        if (partyId && l.partyId !== partyId) continue;
        out.push({
          docId: e.id,
          docNumber: e.number,
          docDate: e.date,
          sourceKind: e.sourceRef?.kind,
          description: l.description ?? e.description,
          debit: l.debit,
          credit: l.credit,
        });
      }
    }
    return out.sort((a, b) => b.docDate.localeCompare(a.docDate));
  },
);

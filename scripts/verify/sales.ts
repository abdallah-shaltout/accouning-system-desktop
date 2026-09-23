/**
 * Sales / purchases / cash invariants — docs/v2/02-accounting-review.md §4 items 5 and 10, plus
 * general document-count and cash/bank/VAT diagnostics.
 */
import { db } from '../../src/mocks/db';
import { check, closeEnough, glBalance, ok, round2, todo, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 5 (docs/v2/02-accounting-review.md §4, now real — was TODO'd to Phase 3): output VAT GL for a
  // period = Σ VAT on invoice lines − Σ VAT on credit-note lines; input VAT the same for purchases/
  // debit notes. `invoice.taxAmount`/`refund.taxAmount` ARE the Σ-of-per-line VAT now
  // (src/mocks/backend/sales.ts's `prepareSale` posts `totals.vat` — the sum computeInvoiceTotals
  // produces per line — as the `vatOutput` credit), so summing them across every posted document
  // must equal the vatOutput/vatInput ledger balances exactly, for ALL TIME (no period filter — the
  // ledger balance itself has none either).
  // v2 phase 9 (docs/v2/10 §2): `taxAmount` (like every other total) is in the DOCUMENT's own
  // currency — an FC invoice's `taxAmount` must convert to base at its own `exchangeRate` before
  // summing against the (always-base) `vatOutput` ledger. Refunds are always in the base currency
  // (Phase 9 doesn't extend FC to returns — a smaller, explicit deferral, see the phase report).
  const invoiceVatBase = (i: (typeof db.invoices)[number]) => (i.currency && i.exchangeRate ? round2(i.taxAmount * i.exchangeRate) : i.taxAmount);
  const outputVatFromDocs = round2(
    db.invoices.reduce((a, i) => a + invoiceVatBase(i), 0) - db.refunds.reduce((a, r) => a + r.taxAmount, 0),
  );
  const outputVatLedger = -glBalance('vatOutput'); // vatOutput is credit-normal; glBalance is debit − credit
  results.push(
    check(
      closeEnough(outputVatFromDocs, outputVatLedger, 0.01),
      `output VAT: Σ invoice lines − Σ credit-note lines (${outputVatFromDocs}) = vatOutput GL (${outputVatLedger})`,
    ),
  );

  // v2 phase 8 (docs/v2/09 §1 "VAT", review E3): a non-VAT supplier's receipt never debits vatInput
  // at all — the VAT is added to cost instead — so it (and its debit notes) are excluded from both
  // sides of this check; `po.taxAmount` on a RECEIVED order is the VAT actually billed by the order,
  // which is what was (or wasn't) posted to vatInput at receiving time. Tax-invoice expenses (§4:
  // "Dr expense[cc] + vatInput") also claim input VAT and must be added in, same as purchases.
  const recoverablePos = db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && !p.vatNotRecoverable);
  const recoverablePoIds = new Set(recoverablePos.map((p) => p.id));
  const inputVatFromDocs = round2(
    recoverablePos.reduce((a, p) => a + p.taxAmount, 0) -
      db.purchaseReturns.filter((r) => recoverablePoIds.has(r.purchaseOrderId)).reduce((a, r) => a + r.taxAmount, 0) +
      db.expenses.reduce((a, e) => a + e.taxAmount, 0),
  );
  const inputVatLedger = glBalance('vatInput'); // vatInput is debit-normal
  results.push(
    check(
      closeEnough(inputVatFromDocs, inputVatLedger, 0.01),
      `input VAT: Σ purchase invoices − Σ debit notes + Σ tax-invoice expenses (${inputVatFromDocs}) = vatInput GL (${inputVatLedger})`,
    ),
  );

  // 10 (docs/v2/02-accounting-review.md §4, docs/v2/09-purchases-payments-expenses.md §2 "Card
  // settlement"): card and wallet clearing balances match the unsettled tenders. Phase 8 built the
  // card-settlement voucher (Dr bank + Dr cardFees / Cr cardClearing-or-walletClearing), which clears
  // exactly the gross amount of its selected day×method groups out of the clearing account — so the
  // GL balance is Σ every card/wallet tender EVER posted, minus Σ every settled group's gross amount
  // (never a refund: the mock always refunds via `settlementAccountFor`, which never resolves to
  // cardClearing/walletClearing, so refunds never touch these accounts).
  const cardMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'cardClearing').map((m) => m.id));
  const walletMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'walletClearing').map((m) => m.id));
  const tenderSum = (ids: Set<string>) =>
    round2(db.invoices.reduce((a, i) => a + (i.tenders ?? []).filter((t) => ids.has(t.paymentMethodId)).reduce((s, t) => s + t.amount, 0), 0));
  const settledSum = (ids: Set<string>) =>
    round2(db.cardSettlements.reduce((a, s) => a + s.groups.filter((g) => ids.has(g.paymentMethodId)).reduce((sub, g) => sub + g.amount, 0), 0));

  const cardTenders = round2(tenderSum(cardMethodIds) - settledSum(cardMethodIds));
  const cardLedger = glBalance('cardClearing');
  results.push(check(closeEnough(cardTenders, cardLedger, 0.01), `card clearing: unsettled card tenders (${cardTenders}) = cardClearing GL (${cardLedger})`));

  const walletTenders = round2(tenderSum(walletMethodIds) - settledSum(walletMethodIds));
  const walletLedger = glBalance('walletClearing');
  results.push(check(closeEnough(walletTenders, walletLedger, 0.01), `wallet clearing: unsettled wallet tenders (${walletTenders}) = walletClearing GL (${walletLedger})`));

  const todayKey = new Date().toDateString();
  const todayInvoices = db.invoices.filter((i) => new Date(i.date).toDateString() === todayKey).length;
  const statusCounts = Object.entries(
    db.invoices.reduce((a: Record<string, number>, i) => {
      const key = `${i.status}/${i.paymentStatus}`;
      a[key] = (a[key] ?? 0) + 1;
      return a;
    }, {}),
  );
  results.push(
    ok(
      `${db.invoices.length} invoices (${todayInvoices} today), ${db.refunds.length} refunds, ${db.purchaseOrders.length} POs — statuses: ${statusCounts.map(([k, v]) => `${k}=${v}`).join(', ')}`,
    ),
  );

  const poStatuses = Object.entries(
    db.purchaseOrders.reduce((a: Record<string, number>, p) => {
      a[p.status] = (a[p.status] ?? 0) + 1;
      return a;
    }, {}),
  );
  results.push(ok(`PO statuses: ${poStatuses.map(([k, v]) => `${k}=${v}`).join(', ') || 'none'}`));

  const cash = glBalance('cash');
  const bank = glBalance('bank');
  const vatOut = -glBalance('vatOutput');
  const vatIn = glBalance('vatInput');
  results.push(ok(`cash ${cash}, bank ${bank}, VAT out ${vatOut}, VAT in ${vatIn}`));

  // 11 (docs/v2/02-accounting-review.md §4, docs/v2/06-sales-and-pos.md §5 "Close"): every closed
  // shift's expected cash − counted cash must equal the posted over/short amount exactly.
  // `src/mocks/backend/shifts.ts`'s `closeShift` posts Dr cash/Cr cashOver when counted > expected,
  // Dr cashShort/Cr cash when counted < expected, and stores `expectedCash`/`variance` on the row —
  // this just re-derives `counted − expected` from the stored fields and checks it matches `variance`
  // exactly, for every closed shift.
  const closedShifts = db.shifts.filter((s) => s.status === 'CLOSED');
  if (!closedShifts.length) {
    results.push(todo('invariant 11 (shift variance): no closed shifts yet — nothing to check', 7));
  } else {
    let shiftFail = false;
    for (const s of closedShifts) {
      const expected = s.expectedCash ?? 0;
      const counted = s.countedCash ?? 0;
      const storedVariance = s.variance ?? 0;
      const derivedVariance = round2(counted - expected);
      if (!closeEnough(storedVariance, derivedVariance, 0.01)) {
        results.push(check(false, `shift ${s.number}: stored variance (${storedVariance}) ≠ counted − expected (${derivedVariance})`));
        shiftFail = true;
      }
    }
    if (!shiftFail) {
      results.push(
        ok(`shift variance: all ${closedShifts.length} closed shift(s)' expected − counted match the posted over/short amount exactly`),
      );
    }
  }

  return results;
}

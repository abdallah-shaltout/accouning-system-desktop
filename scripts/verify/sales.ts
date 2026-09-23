/**
 * Sales / purchases / cash invariants — docs/v2/02-accounting-review.md §4 items 5 and 10, plus
 * general document-count and cash/bank/VAT diagnostics.
 */
import { db } from '../../src/mocks/db';
import { check, closeEnough, glBalance, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 5 (docs/v2/02-accounting-review.md §4, now real — was TODO'd to Phase 3): output VAT GL for a
  // period = Σ VAT on invoice lines − Σ VAT on credit-note lines; input VAT the same for purchases/
  // debit notes. `invoice.taxAmount`/`refund.taxAmount` ARE the Σ-of-per-line VAT now
  // (src/mocks/backend/sales.ts's `prepareSale` posts `totals.vat` — the sum computeInvoiceTotals
  // produces per line — as the `vatOutput` credit), so summing them across every posted document
  // must equal the vatOutput/vatInput ledger balances exactly, for ALL TIME (no period filter — the
  // ledger balance itself has none either).
  const outputVatFromDocs = round2(
    db.invoices.reduce((a, i) => a + i.taxAmount, 0) - db.refunds.reduce((a, r) => a + r.taxAmount, 0),
  );
  const outputVatLedger = -glBalance('vatOutput'); // vatOutput is credit-normal; glBalance is debit − credit
  results.push(
    check(
      closeEnough(outputVatFromDocs, outputVatLedger, 0.01),
      `output VAT: Σ invoice lines − Σ credit-note lines (${outputVatFromDocs}) = vatOutput GL (${outputVatLedger})`,
    ),
  );

  const inputVatFromDocs = round2(
    db.purchaseOrders.filter((p) => p.status === 'CONFIRMED').reduce((a, p) => a + p.taxAmount, 0) -
      db.purchaseReturns.reduce((a, r) => a + r.taxAmount, 0),
  );
  const inputVatLedger = glBalance('vatInput'); // vatInput is debit-normal
  results.push(
    check(
      closeEnough(inputVatFromDocs, inputVatLedger, 0.01),
      `input VAT: Σ purchase invoices − Σ debit notes (${inputVatFromDocs}) = vatInput GL (${inputVatLedger})`,
    ),
  );

  // 10 (docs/v2/02-accounting-review.md §4, docs/v2/09-purchases-payments-expenses.md §2 "Card
  // settlement"): card and wallet clearing balances match the unsettled tenders. No settlement
  // voucher exists yet (that's Phase 8 — see the TODO in src/mocks/backend/sales.ts), so EVERY
  // card/wallet tender posted so far is unsettled by construction: Σ card/wallet tenders on every
  // sale (net of any cash-back paid from the same clearing account on a refund — the mock always
  // refunds via `settlementAccountFor`, which never resolves to cardClearing/walletClearing, so
  // refunds never touch these accounts) must equal the cardClearing/walletClearing GL balance.
  const cardMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'cardClearing').map((m) => m.id));
  const walletMethodIds = new Set(db.paymentMethods.filter((m) => m.accountRole === 'walletClearing').map((m) => m.id));
  const tenderSum = (ids: Set<string>) =>
    round2(db.invoices.reduce((a, i) => a + (i.tenders ?? []).filter((t) => ids.has(t.paymentMethodId)).reduce((s, t) => s + t.amount, 0), 0));

  const cardTenders = tenderSum(cardMethodIds);
  const cardLedger = glBalance('cardClearing');
  results.push(check(closeEnough(cardTenders, cardLedger, 0.01), `card clearing: Σ card tenders (${cardTenders}) = cardClearing GL (${cardLedger})`));

  const walletTenders = tenderSum(walletMethodIds);
  const walletLedger = glBalance('walletClearing');
  results.push(check(closeEnough(walletTenders, walletLedger, 0.01), `wallet clearing: Σ wallet tenders (${walletTenders}) = walletClearing GL (${walletLedger})`));

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

  return results;
}

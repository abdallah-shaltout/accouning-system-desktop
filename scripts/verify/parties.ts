/**
 * Party (customers/suppliers) invariants — docs/v2/02-accounting-review.md §4 items 3, 6.
 */
import { db } from '../../src/mocks/db';
import { customerBalance, customerStatement, supplierBalance, supplierStatement } from '../../src/mocks/backend/balances';
import { unallocatedCreditFor } from '../../src/mocks/backend/payments';
import { check, closeEnough, glBalance, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // 3. GL(receivable) = Σ customer sub-ledgers; GL(payable) = Σ supplier sub-ledgers. Holds
  // structurally now (C2): every AR/AP posting tags partyKind/partyId, and balances are computed
  // from those same ledger lines (src/mocks/backend/balances.ts), not from summing documents.
  // (Per-currency split is Phase 9 territory — single base currency for now.)
  const arGl = glBalance('receivable');
  const arSum = round2(db.customers.reduce((a, c) => a + customerBalance(c.id), 0));
  results.push(check(closeEnough(arGl, arSum), `AR GL (${arGl}) matches Σ customer balances (${arSum})`));

  const apGl = -glBalance('payable');
  const apSum = round2(db.suppliers.reduce((a, s) => a + supplierBalance(s.id), 0));
  results.push(check(closeEnough(apGl, apSum), `AP GL (${apGl}) matches Σ supplier balances (${apSum})`));

  // 6. For every party: Σ document outstanding − unallocated credit ± opening balance = sub-ledger
  // balance. Unallocated credit / opening balance aren't modeled until Phase 4/5 — verified here as
  // "statement's last running balance equals the party's balance function" (what already applies).
  const custMismatch = db.customers.filter((c) => {
    const st = customerStatement(c.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, customerBalance(c.id));
  });
  results.push(check(custMismatch.length === 0, `every customer statement's running balance matches customerBalance() (${custMismatch.length} mismatched)`));

  const supMismatch = db.suppliers.filter((s) => {
    const st = supplierStatement(s.id);
    const last = st.at(-1)?.balance ?? 0;
    return !closeEnough(last, supplierBalance(s.id));
  });
  results.push(check(supMismatch.length === 0, `every supplier statement's running balance matches supplierBalance() (${supMismatch.length} mismatched)`));

  // 6 (docs/v2/02-accounting-review.md §4 item 6, C1's allocation fix — Phase 4): for every party,
  // Σ document outstanding − unallocated credit ± opening balance = sub-ledger balance. Opening
  // balances aren't posted yet (Phase 5's wizard), so that term is 0 here; unallocated credit now
  // comes from `payments.ts`'s allocation sub-ledger (Σ amount − Σ allocations per payment).
  //
  // Outstanding is summed *unclamped* here (total − AR-reducing credit notes − paid, without the
  // `Math.max(0, …)` that `invoiceOutstanding`/`purchaseOutstanding` apply for display/"open
  // documents" purposes): a document that a credit note pushed below zero (e.g. a purchase return
  // posted as a supplier credit after the PO was already paid in full) is a real AP/AR reduction
  // the ledger already reflects, and clamping it to 0 here would make this invariant fail even
  // though `Σ debit − Σ credit` on the control account is exactly right. Only COMPLETED/CONFIRMED
  // documents carry a receivable/payable balance at all (drafts/cancels never posted).
  //
  // A sales refund's AR impact is `settledToReceivable`, not its full `grandTotal`/`refundedAmount`
  // — the rest (`cashBack`) never touched AR, so it must not be subtracted here either (mirrors
  // `sales.ts`'s posting: `receivable credit settledToReceivable`, not `refund.grandTotal`).
  const arReducedByRefunds = (invoiceId: string) => db.refunds.filter((r) => r.invoiceId === invoiceId).reduce((a, r) => a + r.settledToReceivable, 0);
  const custAllocationMismatch = db.customers.filter((c) => {
    const outstanding = round2(
      db.invoices
        .filter((i) => i.customerId === c.id && i.status !== 'DRAFT')
        .reduce((a, i) => a + (i.grandTotal - arReducedByRefunds(i.id) - i.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('customer', c.id);
    return !closeEnough(round2(outstanding - credit), customerBalance(c.id));
  });
  results.push(check(custAllocationMismatch.length === 0, `every customer: Σoutstanding − unallocated credit = customerBalance() (${custAllocationMismatch.length} mismatched)`));

  // Unlike a sales refund's cashBack (always paid through the settlement account), a purchase
  // return's cashBack with `refundMethod: 'credit'` is ALSO posted as a payable debit
  // (src/mocks/backend/purchases.ts's `recordPurchaseReturn`: `payable debit settledToPayable` +
  // `payable debit cashBack` when credit) — so the AP reduction is the return's full `grandTotal`
  // in that case, not just `settledToPayable`.
  const apReducedByReturns = (poId: string) =>
    db.purchaseReturns.filter((r) => r.purchaseOrderId === poId).reduce((a, r) => a + (r.refundMethod === 'credit' ? r.grandTotal : r.settledToPayable), 0);
  const supAllocationMismatch = db.suppliers.filter((s) => {
    const outstanding = round2(
      db.purchaseOrders
        .filter((p) => p.supplierId === s.id && p.status === 'RECEIVED')
        .reduce((a, p) => a + (p.grandTotal - apReducedByReturns(p.id) - p.paidAmount), 0),
    );
    const credit = unallocatedCreditFor('supplier', s.id);
    return !closeEnough(round2(outstanding - credit), supplierBalance(s.id));
  });
  results.push(check(supAllocationMismatch.length === 0, `every supplier: Σoutstanding − unallocated credit = supplierBalance() (${supAllocationMismatch.length} mismatched)`));

  const totalAllocated = round2(db.payments.reduce((a, p) => a + p.allocations.reduce((b, al) => b + al.amount, 0), 0));
  const multiAllocation = db.payments.filter((p) => p.allocations.length > 1).length;
  results.push(ok(`${db.customers.length} customers, ${db.suppliers.length} suppliers, ${db.payments.length} payments (${multiAllocation} multi-document, Σallocated ${totalAllocated})`));

  return results;
}

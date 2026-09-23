/**
 * v2 phase 9 invariants (docs/v2/10-branches-currencies-cost-centers.md): the realized-FX worked
 * example pinned exactly, plus branch/cost-center/currency structural checks that only start
 * applying once the seed actually exercises them (the seed always does, per this phase's checklist
 * — "every invariant must hold true even with the new USD customer and second branch in the seed").
 */
import { db } from '../../src/mocks/db';
import { customerBalanceFc } from '../../src/mocks/backend/balances';
import { check, closeEnough, ok, round2, type Result } from './shared';

export function run(): Result[] {
  const results: Result[] = [];

  // --- Realized FX worked example (docs/v2/10 §2), pinned to the exact numbers in the doc: -------
  // Invoice USD 1,000 @ 48.50 -> AR 48,500 EGP-equivalent (SAR here); receipt USD 1,000 @ 49.20 ->
  // cash 49,200; Dr Cash(USD) 49,200 / Cr AR[party] 48,500 / Cr 4310 FX gain 700; party ledger in
  // USD nets to exactly 0. `seedBranches9` posts exactly this scenario for `cus-usd-1`/`inv.
  const usdCustomer = db.customers.find((c) => c.id === 'cus-usd-1');
  if (!usdCustomer) {
    results.push(check(false, 'FX worked example: seed customer cus-usd-1 not found'));
  } else {
    const invoice = db.invoices.find((i) => i.customerId === usdCustomer.id && i.currency === 'USD');
    const payment = db.payments.find((p) => p.targetId === usdCustomer.id && p.currency === 'USD');
    const alloc = payment?.allocations.find((a) => a.targetId === invoice?.id);

    results.push(check(!!invoice && invoice.exchangeRate === 48.5 && invoice.grandTotal === 1000, `FX example: invoice is USD 1,000 @ 48.50 (got ${invoice?.grandTotal} @ ${invoice?.exchangeRate})`));
    results.push(check(!!alloc && alloc.amount === 48500, `FX example: AR posting = 48,500 (got ${alloc?.amount})`));
    results.push(check(!!payment && payment.amount === 49200, `FX example: cash posting = 49,200 (got ${payment?.amount})`));
    results.push(check(!!alloc && alloc.fxGainLoss === 700, `FX example: realized FX gain = 700 (got ${alloc?.fxGainLoss})`));

    const fcBalance = customerBalanceFc(usdCustomer.id);
    results.push(check(closeEnough(fcBalance, 0, 0.01), `FX example: party ledger nets to exactly 0 USD (got ${fcBalance})`));

    // The exact journal entry shape: Dr cash / Cr AR 48,500 / Cr FX gain 700, all in one entry.
    const entry = payment ? db.journalEntries.find((e) => e.sourceRef?.kind === 'payment' && e.sourceRef.id === payment.id) : undefined;
    const cashLine = entry?.lines.find((l) => l.debit === 49200);
    const arLine = entry?.lines.find((l) => l.credit === 48500 && l.partyId === usdCustomer.id);
    const fxAccount = db.accounts.find((a) => a.systemRole === 'fxGain');
    const fxLine = entry?.lines.find((l) => l.accountId === fxAccount?.id && l.credit === 700);
    results.push(check(!!entry && !!cashLine && !!arLine && !!fxLine, 'FX example: one journal entry = Dr Cash 49,200 / Cr AR 48,500 / Cr FX gain 700'));
  }

  // --- Branches (docs/v2/10 §1) --------------------------------------------------------------
  results.push(check(db.branches.length >= 2, `at least 2 branches in the seed (${db.branches.length})`));
  const branchesWithCash = db.branches.filter((b) => b.cashAccountId && db.accounts.some((a) => a.id === b.cashAccountId));
  results.push(check(branchesWithCash.length === db.branches.length, `every branch has a real cash-drawer account (${branchesWithCash.length}/${db.branches.length})`));
  const branchesWithCc = db.branches.filter((b) => b.costCenterId && db.costCenters.some((c) => c.id === b.costCenterId));
  results.push(check(branchesWithCc.length === db.branches.length, `every branch has its own cost center (${branchesWithCc.length}/${db.branches.length})`));

  // Every journal line's branchId resolves to a real branch (no dangling ids from before Phase 9).
  const allLines = db.journalEntries.flatMap((e) => e.lines);
  const branchIds = new Set(db.branches.map((b) => b.id));
  const danglingBranch = allLines.filter((l) => l.branchId && !branchIds.has(l.branchId));
  results.push(check(danglingBranch.length === 0, `every journal line's branchId resolves to a real branch (${danglingBranch.length} dangling)`));

  // Stock transfer (§4): the seed doesn't post one by default (kept optional/UI-driven), but if any
  // exist, GL(inventory) + GL(inventoryInTransit) together must still equal Σ product.stockValue —
  // a transfer only ever reclassifies value between those two roles, never creates/destroys it.
  if (db.stockTransfers.length) {
    const invAcc = db.accounts.find((a) => a.systemRole === 'inventory')!;
    const transitAcc = db.accounts.find((a) => a.systemRole === 'inventoryInTransit')!;
    const bal = (id: string) => round2(allLines.filter((l) => l.accountId === id).reduce((a, l) => a + l.debit - l.credit, 0));
    const combined = round2(bal(invAcc.id) + bal(transitAcc.id));
    const stockSum = round2(db.products.reduce((a, p) => a + (p.type === 'product' ? p.stockValue : 0), 0));
    results.push(check(closeEnough(combined, stockSum, 0.01), `inventory + in-transit GL (${combined}) matches Σ product.stockValue (${stockSum}) across ${db.stockTransfers.length} transfer(s)`));
  } else {
    results.push(ok('no stock transfers in the seed — inventory+transit invariant vacuously holds'));
  }

  // --- Cost centers (§3) ------------------------------------------------------------------------
  results.push(check(db.costCenters.length >= 3, `at least 3 cost centers (2 branch + 1 other) in the seed (${db.costCenters.length})`));
  const ccIds = new Set(db.costCenters.map((c) => c.id));
  const danglingCc = allLines.filter((l) => l.costCenterId && !ccIds.has(l.costCenterId));
  results.push(check(danglingCc.length === 0, `every journal line's costCenterId resolves to a real cost center (${danglingCc.length} dangling)`));

  // The seed's rent split (60/40 across two branch cost centers) sums back to the original amount.
  const splitEntry = db.journalEntries.find((e) => e.description.includes('موزع على الفروع'));
  if (splitEntry) {
    const rentLines = splitEntry.lines.filter((l) => l.debit > 0 && l.costCenterId);
    const sumSplit = round2(rentLines.reduce((a, l) => a + l.debit, 0));
    results.push(check(rentLines.length === 2 && sumSplit === 5000, `cost-center split: 2 lines summing to the original 5,000 (${rentLines.length} lines, Σ${sumSplit})`));
  } else {
    results.push(check(false, 'cost-center split demo entry not found in the seed'));
  }

  // --- Currencies (§2) --------------------------------------------------------------------------
  const usd = db.currencies.find((c) => c.code === 'USD');
  results.push(check(!!usd?.active, 'USD is an active enabled currency'));
  results.push(check(db.exchangeRates.filter((r) => r.currency === 'USD').length >= 2, `at least 2 USD exchange-rate rows (${db.exchangeRates.filter((r) => r.currency === 'USD').length})`));

  return results;
}

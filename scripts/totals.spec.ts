/**
 * Pins `computeInvoiceTotals` (modules/invoices/helpers/totals.ts) against the exact algorithm in
 * docs/v2/06-sales-and-pos.md §3, including the worked-example numbers.
 *
 * No test runner is configured in this project (no vitest/jest in package.json) — this is a plain
 * runnable assertion script, following the `scripts/verify/*.ts` convention (bun-executable, no
 * framework). It is NOT wired into `bun run verify:mocks` (that suite checks the seeded mock DB's
 * ledger invariants, not this module's pure math) — run it directly:
 *
 *   bun run scripts/totals.spec.ts
 */
import { computeInvoiceTotals, round2, type TotalsLineInput } from '../src/modules/invoices/helpers/totals';

let passed = 0;
let failed = 0;

function eq(actual: number, expected: number, label: string) {
  const ok = Math.abs(actual - expected) < 0.001;
  if (ok) {
    passed++;
  } else {
    failed++;
    console.error(`FAIL  ${label}: expected ${expected}, got ${actual}`);
  }
}

function ok(cond: boolean, label: string) {
  if (cond) {
    passed++;
  } else {
    failed++;
    console.error(`FAIL  ${label}`);
  }
}

const S: TotalsLineInput['tax'] = { rate: 15, category: 'S' };
const Z: TotalsLineInput['tax'] = { rate: 0, category: 'Z' };

// ---------------------------------------------------------------------------------------------
// 1. The worked example from docs/v2/06-sales-and-pos.md §3 (inclusive prices, line B zero-rated).
// ---------------------------------------------------------------------------------------------
{
  const lines: TotalsLineInput[] = [
    { qty: 2, unitPrice: 57.5, discount: 10, discountIsPct: true, tax: S },
    { qty: 1, unitPrice: 23.0, tax: Z },
  ];
  const r = computeInvoiceTotals(lines, { amount: 5.0 }, true);

  eq(r.lines[0].L, 115, 'worked example: line A L');
  eq(r.lines[0].lineDiscount, 11.5, 'worked example: line A discount');
  eq(r.lines[0].Lprime, 103.5, "worked example: line A L'");
  eq(r.lines[1].Lprime, 23.0, "worked example: line B L'");
  eq(r.subTotalAfterLineDiscounts, 126.5, 'worked example: sum L');
  eq(r.invoiceDiscountAmount, 5.0, 'worked example: H');
  eq(r.lines[0].invoiceDiscountShare, 4.09, 'worked example: line A discount share');
  eq(r.lines[1].invoiceDiscountShare, 0.91, 'worked example: line B discount share');
  eq(r.lines[0].gross, 99.41, 'worked example: line A gross');
  eq(r.lines[1].gross, 22.09, 'worked example: line B gross');
  eq(r.gross, 121.5, 'worked example: TOTAL gross');
  eq(r.net, 108.53, 'worked example: TOTAL net');
  eq(r.vat, 12.97, 'worked example: TOTAL vat (the pinned trio: 108.53 / 12.97 / 121.50)');
  ok(
    Math.abs(r.gross - 121.5) < 0.001 && Math.abs(r.net - 108.53) < 0.001 && Math.abs(r.vat - 12.97) < 0.001,
    'worked example: exact trio net=108.53 vat=12.97 gross=121.50',
  );

  // Σhᵢ = H exactly.
  const sumShares = r.lines.reduce((a, l) => a + l.invoiceDiscountShare, 0);
  eq(round2(sumShares), 5.0, 'worked example: Σ invoice-discount shares = H exactly');
}

// ---------------------------------------------------------------------------------------------
// 2. 0 qty line — contributes nothing, gets no discount share, doesn't blow up the spread.
// ---------------------------------------------------------------------------------------------
{
  const lines: TotalsLineInput[] = [
    { qty: 0, unitPrice: 100, tax: S },
    { qty: 2, unitPrice: 50, tax: S },
  ];
  const r = computeInvoiceTotals(lines, { pct: 10 }, false);
  eq(r.lines[0].L, 0, '0 qty: line L is 0');
  eq(r.lines[0].invoiceDiscountShare, 0, '0 qty: gets no discount share');
  eq(r.lines[1].Ldoubleprime, 90, '0 qty: all the 10-unit discount lands on the other line (100 - 10%)');
  eq(r.gross, round2(90 * 1.15), '0 qty: total gross is just the non-zero line, taxed');
}

// ---------------------------------------------------------------------------------------------
// 3. 100% line discount — the line nets to 0 and takes no share of the invoice discount.
// ---------------------------------------------------------------------------------------------
{
  const lines: TotalsLineInput[] = [
    { qty: 1, unitPrice: 200, discount: 100, discountIsPct: true, tax: S },
    { qty: 1, unitPrice: 50, tax: S },
  ];
  const r = computeInvoiceTotals(lines, { pct: 20 }, false);
  eq(r.lines[0].Lprime, 0, '100% discount: line nets to 0');
  eq(r.lines[0].invoiceDiscountShare, 0, '100% discount: gets no invoice-discount share (0 weight)');
  eq(r.lines[1].Ldoubleprime, 40, '100% discount: the other line absorbs the full 20% (50 - 10)');
  eq(r.net, 40, '100% discount: net is only the surviving line');
}

// ---------------------------------------------------------------------------------------------
// 4. Mixed tax categories (S / Z / E / O) — each keeps its own rate and groups separately.
// ---------------------------------------------------------------------------------------------
{
  const E: TotalsLineInput['tax'] = { rate: 0, category: 'E' };
  const O: TotalsLineInput['tax'] = { rate: 0, category: 'O' };
  const lines: TotalsLineInput[] = [
    { qty: 1, unitPrice: 100, tax: S },
    { qty: 1, unitPrice: 100, tax: Z },
    { qty: 1, unitPrice: 100, tax: E },
    { qty: 1, unitPrice: 100, tax: O },
  ];
  const r = computeInvoiceTotals(lines, undefined, false);
  eq(r.vat, 15, 'mixed categories: only the S line contributes VAT');
  ok(r.vatByCategory.length === 4, `mixed categories: 4 distinct (category, rate) groups (got ${r.vatByCategory.length})`);
  const sGroup = r.vatByCategory.find((g) => g.category === 'S');
  ok(!!sGroup && Math.abs(sGroup.vat - 15) < 0.001, 'mixed categories: S group carries all the VAT');
  for (const cat of ['Z', 'E', 'O'] as const) {
    const g = r.vatByCategory.find((x) => x.category === cat);
    ok(!!g && g.vat === 0, `mixed categories: ${cat} group has 0 VAT`);
  }
}

// ---------------------------------------------------------------------------------------------
// 5. Proportional spread when one line is 0 (0 qty / fully discounted) among several lines.
// ---------------------------------------------------------------------------------------------
{
  const lines: TotalsLineInput[] = [
    { qty: 0, unitPrice: 999, tax: S }, // weight 0
    { qty: 1, unitPrice: 30, tax: S },
    { qty: 1, unitPrice: 70, tax: S },
  ];
  const r = computeInvoiceTotals(lines, { amount: 10 }, false);
  eq(r.lines[0].invoiceDiscountShare, 0, 'zero-weight line: no discount share at all');
  eq(r.lines[1].invoiceDiscountShare, 3, 'zero-weight line: remaining lines split 10 as 30:70 → 3');
  eq(r.lines[2].invoiceDiscountShare, 7, 'zero-weight line: remaining lines split 10 as 30:70 → 7');
  const sumShares = r.lines.reduce((a, l) => a + l.invoiceDiscountShare, 0);
  eq(round2(sumShares), 10, 'zero-weight line: shares still sum to H exactly');
}

// ---------------------------------------------------------------------------------------------
// 6. Exclusive vs inclusive give the same net for the same *effective* price.
//    Exclusive @ 100 net + 15% VAT = 115 gross. Inclusive @ 115 gross should net back to 100.
// ---------------------------------------------------------------------------------------------
{
  const exclusive = computeInvoiceTotals([{ qty: 1, unitPrice: 100, tax: S }], undefined, false);
  const inclusive = computeInvoiceTotals([{ qty: 1, unitPrice: 115, tax: S }], undefined, true);
  eq(exclusive.net, 100, 'exclusive/inclusive parity: exclusive net');
  eq(inclusive.net, 100, 'exclusive/inclusive parity: inclusive net (same effective price)');
  eq(exclusive.gross, inclusive.gross, 'exclusive/inclusive parity: same gross');
  eq(exclusive.vat, inclusive.vat, 'exclusive/inclusive parity: same VAT');
}

// ---------------------------------------------------------------------------------------------
// 7. Custom price: listPrice is carried for audit only, never fed into the math.
// ---------------------------------------------------------------------------------------------
{
  const r = computeInvoiceTotals([{ qty: 1, unitPrice: 80, listPrice: 100, tax: S }], undefined, false);
  eq(r.lines[0].L, 80, 'custom price: L uses unitPrice, not listPrice');
  eq(r.net, 80, 'custom price: net uses unitPrice, not listPrice');
}


console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);

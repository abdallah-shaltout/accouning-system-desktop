import type { TaxCategory } from '@/modules/settings/types';
import type { PaymentStatus } from '../types';

import { round2 } from '@/modules/core/helpers/numbers';

/** Re-exported for existing importers — the one rounding rule lives in core/helpers/numbers.ts (D3). */
export { round2 };

// =================================================================================================
// v2 pricing/discount/VAT engine — docs/v2/06-sales-and-pos.md §3 "Pricing, discount & VAT math",
// implemented line for line, algorithm step for algorithm step. This is the single source of truth
// for sales AND purchases (same steps, different tax direction) — `src/mocks/backend/sales.ts` is
// the only caller in this phase's scope, but the shape is generic on purpose (docs/v2/06 §3 "Purchases:
// use the same steps with the purchase tax and the supplier's price mode.").
//
// Worked example pinned by totals.spec (docs/v2/06 §3, inclusive prices, line B zero-rated):
//   Line A: 2 × 57.50, 10% line discount, tax S 15%
//   Line B: 1 × 23.00, tax Z 0%
//   Invoice discount: 5.00 flat, spread proportionally over L' (103.50 / 23.00)
//   → net 108.53, VAT 12.97, gross 121.50 exactly.
// =================================================================================================

/** Minimal shape `computeInvoiceTotals` needs from a tax — see `modules/settings/types` `Tax`. */
export interface TotalsTax {
  rate: number;
  category: TaxCategory;
}

export interface TotalsLineInput {
  qty: number;
  /** The price actually charged (custom-price override already applied). */
  unitPrice: number;
  /** Kept only for audit/reporting — never fed back into the math (docs/v2/06 §3 "Custom price"). */
  listPrice?: number;
  /** Line discount: % of `L` (`discountIsPct: true`) or a flat amount. */
  discount?: number;
  discountIsPct?: boolean;
  /** 0-rated / no tax at all when omitted (e.g. a line with no tax assigned yet). */
  tax?: TotalsTax;
}

export interface TotalsLineResult {
  /** Step 1: qty × unitPrice, rounded. */
  L: number;
  /** Step 2: line discount amount. */
  lineDiscount: number;
  /** Step 2: L − lineDiscount. */
  Lprime: number;
  /** Step 3: this line's share of the invoice discount (largest-remainder rounded so Σh = H exactly). */
  invoiceDiscountShare: number;
  /** Step 3: L' − share. The final taxable/gross base for step 4, before VAT is split out. */
  Ldoubleprime: number;
  net: number;
  vat: number;
  gross: number;
  taxRate: number;
  taxCategory: TaxCategory;
}

export interface VatCategoryTotal {
  category: TaxCategory;
  rate: number;
  net: number;
  vat: number;
}

export interface InvoiceTotalsResult {
  lines: TotalsLineResult[];
  /** Σ L' — the base the invoice discount is spread over. */
  subTotalAfterLineDiscounts: number;
  /** The money value of the invoice-level discount (H). */
  invoiceDiscountAmount: number;
  net: number;
  vat: number;
  gross: number;
  /** Grouped by (category, rate) — docs/v2/06 §3 step 5, feeds the VAT report's per-box breakdown. */
  vatByCategory: VatCategoryTotal[];
}

export type InvoiceDiscountInput = { pct: number } | { amount: number } | undefined;

/**
 * Spread `total` over `weights` in proportion, with "largest remainder" rounding so the shares sum
 * to exactly `round2(total)` — docs/v2/06 §3 step 3. Lines with weight 0 (e.g. a fully-discounted
 * line, or 0 qty) always get a 0 share, never a stray halala from rounding.
 */
function spreadProportionally(total: number, weights: number[]): number[] {
  const target = round2(total);
  const weightSum = weights.reduce((a, w) => a + w, 0);
  if (target === 0 || weightSum <= 0) return weights.map(() => 0);

  // Exact (unrounded) share per line, then round2 each — this is the "proportional" part.
  const raw = weights.map((w) => (target * w) / weightSum);
  const rounded = raw.map((r) => round2(r));
  let diff = round2(target - rounded.reduce((a, r) => a + r, 0));

  if (diff !== 0) {
    // Distribute the leftover halalas to the lines with the largest fractional remainders (or
    // smallest, when diff < 0 and we need to take a halala back), one halala at a time — this is
    // what makes Σ hᵢ = H exactly regardless of how the rounding fell.
    const remainders = raw.map((r, i) => ({ i, remainder: r - rounded[i] }));
    const step = diff > 0 ? 0.01 : -0.01;
    const order = [...remainders].sort((a, b) => (diff > 0 ? b.remainder - a.remainder : a.remainder - b.remainder));
    let cursor = 0;
    let guard = order.length * 2 + 4; // rounding leftovers are at most a few halalas; this just bounds the loop
    while (Math.abs(diff) > 0.001 && guard-- > 0) {
      const target_i = order[cursor % order.length].i;
      if (weights[target_i] > 0) {
        rounded[target_i] = round2(rounded[target_i] + step);
        diff = round2(diff - step);
      }
      cursor++;
    }
  }
  return rounded;
}

/**
 * The v2 pricing/discount/VAT engine — docs/v2/06-sales-and-pos.md §3, followed step by step:
 *   1. Lᵢ = round2(qtyᵢ × unitPriceᵢ)
 *   2. line discount → L'ᵢ = Lᵢ − dᵢ
 *   3. invoice discount H spread proportionally over L'ᵢ, largest-remainder rounded → L''ᵢ = L'ᵢ − hᵢ
 *   4. VAT per line, exclusive or inclusive of L''ᵢ depending on `pricesIncludeTax`
 *   5. invoice totals = Σ lines; vatByCategory groups by (category, rate)
 */
export function computeInvoiceTotals(
  lines: TotalsLineInput[],
  invoiceDiscount: InvoiceDiscountInput,
  pricesIncludeTax: boolean,
): InvoiceTotalsResult {
  // Steps 1-2: per-line amount and line discount.
  const step2 = lines.map((l) => {
    const L = round2(l.qty * l.unitPrice);
    const lineDiscount = round2(l.discountIsPct ? L * ((l.discount ?? 0) / 100) : (l.discount ?? 0));
    const Lprime = round2(Math.max(0, L - lineDiscount));
    return { L, lineDiscount, Lprime };
  });

  // Step 3: invoice discount H, spread proportionally over L'ᵢ with largest-remainder rounding.
  const sumLprime = round2(step2.reduce((a, s) => a + s.Lprime, 0));
  const H = !invoiceDiscount
    ? 0
    : 'pct' in invoiceDiscount
      ? round2(sumLprime * (invoiceDiscount.pct / 100))
      : round2(invoiceDiscount.amount);
  const shares = spreadProportionally(H, step2.map((s) => s.Lprime));

  // Step 4: VAT per line (exclusive or inclusive), step 5: group by (category, rate).
  const vatGroups = new Map<string, VatCategoryTotal>();
  const results: TotalsLineResult[] = lines.map((l, i) => {
    const { L, lineDiscount, Lprime } = step2[i];
    const invoiceDiscountShare = shares[i];
    const Ldoubleprime = round2(Math.max(0, Lprime - invoiceDiscountShare));
    const rate = l.tax?.rate ?? 0;
    const category = l.tax?.category ?? 'O';

    let net: number;
    let vat: number;
    let gross: number;
    if (pricesIncludeTax) {
      gross = Ldoubleprime;
      net = round2(gross / (1 + rate / 100));
      vat = round2(gross - net);
    } else {
      net = Ldoubleprime;
      vat = round2(net * (rate / 100));
      gross = round2(net + vat);
    }

    const key = `${category}:${rate}`;
    const group = vatGroups.get(key) ?? { category, rate, net: 0, vat: 0 };
    group.net = round2(group.net + net);
    group.vat = round2(group.vat + vat);
    vatGroups.set(key, group);

    return { L, lineDiscount, Lprime, invoiceDiscountShare, Ldoubleprime, net, vat, gross, taxRate: rate, taxCategory: category };
  });

  return {
    lines: results,
    subTotalAfterLineDiscounts: sumLprime,
    invoiceDiscountAmount: H,
    net: round2(results.reduce((a, r) => a + r.net, 0)),
    vat: round2(results.reduce((a, r) => a + r.vat, 0)),
    gross: round2(results.reduce((a, r) => a + r.gross, 0)),
    vatByCategory: [...vatGroups.values()],
  };
}

// =================================================================================================
// Legacy v1 engine — kept working, unchanged behavior, for callers outside this phase's surface
// (modules/invoices/controllers/usePosStore.ts, CheckoutModal.vue — the POS cart hasn't been
// migrated to per-line tax categories yet; that's Phase 7's "POS v2" per docs/v2/15-action-plan.md).
// Implemented in terms of the v2 engine above (single category, exclusive-rate semantics) so both
// paths share one rounding implementation.
// =================================================================================================

export interface SaleTotals {
  /** Σ qty × price − line discounts. */
  subTotal: number;
  /** Money value of the overall discount %. */
  discountAmount: number;
  /** subTotal − discountAmount: the VAT base and the Sales-account credit. */
  taxable: number;
  taxAmount: number;
  grandTotal: number;
}

/**
 * Invoice math (prices are VAT-exclusive, one overall discount %, one VAT rate) — the pre-Phase-3
 * POS cart shape. Shared by the POS cart and the mock backend so the preview always matches what
 * gets posted. New code should use `computeInvoiceTotals` instead.
 */
export function computeSaleTotals(
  lines: { qty: number; price: number; discount?: number }[],
  discountRate: number,
  taxRate: number,
): SaleTotals {
  const result = computeInvoiceTotals(
    lines.map((l) => ({ qty: l.qty, unitPrice: l.price, discount: l.discount ?? 0, discountIsPct: false, tax: { rate: taxRate, category: 'S' as const } })),
    { pct: discountRate },
    false,
  );
  return {
    subTotal: result.subTotalAfterLineDiscounts,
    discountAmount: result.invoiceDiscountAmount,
    taxable: round2(result.subTotalAfterLineDiscounts - result.invoiceDiscountAmount),
    taxAmount: result.vat,
    grandTotal: result.gross,
  };
}

export function paymentStatusFor(total: number, paid: number): PaymentStatus {
  if (paid >= total - 0.005) return 'PAID';
  if (paid > 0) return 'PARTIALLY_PAID';
  return 'UNPAID';
}

/** What the customer still owes on an invoice (never negative). */
export function invoiceOutstanding(inv: { grandTotal: number; refundedAmount: number; paidAmount: number }): number {
  return Math.max(0, round2(inv.grandTotal - inv.refundedAmount - inv.paidAmount));
}

/** Cash change to hand back, given what was tendered. */
export function changeDue(total: number, tendered: number): number {
  return Math.max(0, round2(tendered - total));
}

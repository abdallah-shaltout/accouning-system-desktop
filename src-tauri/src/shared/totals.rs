//! `shared::totals` (G-21) — the ONE Rust totals/VAT engine, a behaviour-exact port of
//! `computeInvoiceTotals` (`src/modules/invoices/helpers/totals.ts:126-188`), used by **both**
//! invoices (`pricesIncludeTax` from `settings.prices_include_tax`) and purchases (always
//! `pricesIncludeTax = false` — `src/mocks/backend/purchases.ts:47-72`'s `computePurchaseTotals`
//! is nothing but this same engine with a purchase-tax line mapping in front of it). Never write a
//! second copy of this math for purchases (G-21's whole point) — a purchase caller maps its own
//! line shape into `TotalsLineInput` and calls `compute_invoice_totals` directly, exactly like the
//! mock's `computePurchaseTotals` wraps `computeInvoiceTotals`.
//!
//! Algorithm (docs/v2/06-sales-and-pos.md §3, followed step by step — see the TS file's own
//! worked example, reproduced in this module's tests):
//!   1. `Lᵢ = round2(qtyᵢ × unitPriceᵢ)`
//!   2. line discount → `L'ᵢ = Lᵢ − dᵢ` (never negative)
//!   3. invoice discount `H` spread proportionally over `L'ᵢ`, "largest remainder" rounded so
//!      `Σhᵢ = round2(H)` exactly → `L''ᵢ = L'ᵢ − hᵢ` (never negative)
//!   4. VAT per line, exclusive or inclusive of `L''ᵢ` depending on `prices_include_tax`
//!   5. invoice totals = Σ lines; `vat_by_category` groups by `(category, rate)`
//!
//! Every rounding point uses `round2` (the one rounding rule, D3) — never a second `round`/`floor`/
//! float shortcut. `Decimal` carries the exact value with no binary float noise, so the "largest
//! remainder" distribution below needs no `toPrecision(15)`-style float cleanup the TS side needs.

use std::collections::BTreeMap;

use rust_decimal::Decimal;

use crate::utils::money::round2;

/// `TaxCategory` (`settings/types/index.ts:18`) — the ZATCA category a tax line carries. Matches
/// the `taxes.category` column's stored string exactly (`'S' | 'Z' | 'E' | 'O'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaxCategory {
    /// Standard-rated.
    Standard,
    /// Zero-rated.
    Zero,
    /// Exempt.
    Exempt,
    /// Out of scope / no tax assigned (the engine's own default when a line has no tax).
    Other,
}

impl TaxCategory {
    /// The exact one-letter code stored in `taxes.category` / sent over the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            TaxCategory::Standard => "S",
            TaxCategory::Zero => "Z",
            TaxCategory::Exempt => "E",
            TaxCategory::Other => "O",
        }
    }

    /// The inverse of `as_str()` — unknown codes fall back to `Other`, matching a line with no
    /// resolvable tax category rather than erroring (the mock's own `l.tax?.category ?? 'O'`
    /// default never fails either).
    pub fn from_str_or_other(s: &str) -> Self {
        match s {
            "S" => TaxCategory::Standard,
            "Z" => TaxCategory::Zero,
            "E" => TaxCategory::Exempt,
            _ => TaxCategory::Other,
        }
    }
}

/// `TotalsTax` (`totals.ts:24-27`) — the minimal tax shape the engine needs off a resolved tax
/// row.
#[derive(Debug, Clone, Copy)]
pub struct TotalsTax {
    pub rate: Decimal,
    pub category: TaxCategory,
}

/// `TotalsLineInput` (`totals.ts:29-40`).
#[derive(Debug, Clone)]
pub struct TotalsLineInput {
    pub qty: Decimal,
    /// The price actually charged (custom-price override already applied).
    pub unit_price: Decimal,
    /// Line discount: a percentage of `L` when `discount_is_pct`, else a flat amount.
    pub discount: Decimal,
    pub discount_is_pct: bool,
    /// `None` = 0-rated / no tax at all (a line with no tax assigned yet) — matches the mock's
    /// `l.tax?.rate ?? 0` / `l.tax?.category ?? 'O'` defaults exactly.
    pub tax: Option<TotalsTax>,
}

impl TotalsLineInput {
    /// Convenience constructor for the common case: no discount, no tax.
    pub fn new(qty: Decimal, unit_price: Decimal) -> Self {
        Self { qty, unit_price, discount: Decimal::ZERO, discount_is_pct: false, tax: None }
    }
}

/// `TotalsLineResult` (`totals.ts:42-58`) — every intermediate value the mock keeps per line
/// (needed for line-level display and for the invoice-discount-share audit trail).
#[derive(Debug, Clone, Copy)]
pub struct TotalsLineResult {
    /// Step 1: `qty × unitPrice`, rounded.
    pub l: Decimal,
    /// Step 2: line discount amount.
    pub line_discount: Decimal,
    /// Step 2: `L − lineDiscount`.
    pub l_prime: Decimal,
    /// Step 3: this line's share of the invoice discount (largest-remainder rounded).
    pub invoice_discount_share: Decimal,
    /// Step 3: `L' − share` — the final taxable/gross base for step 4.
    pub l_double_prime: Decimal,
    pub net: Decimal,
    pub vat: Decimal,
    pub gross: Decimal,
    pub tax_rate: Decimal,
    pub tax_category: TaxCategory,
}

/// `VatCategoryTotal` (`totals.ts:60-65`).
#[derive(Debug, Clone, Copy)]
pub struct VatCategoryTotal {
    pub category: TaxCategory,
    pub rate: Decimal,
    pub net: Decimal,
    pub vat: Decimal,
}

/// `InvoiceTotalsResult` (`totals.ts:67-78`).
#[derive(Debug, Clone)]
pub struct InvoiceTotalsResult {
    pub lines: Vec<TotalsLineResult>,
    /// `Σ L'` — the base the invoice discount is spread over.
    pub sub_total_after_line_discounts: Decimal,
    /// The money value of the invoice-level discount (`H`).
    pub invoice_discount_amount: Decimal,
    pub net: Decimal,
    pub vat: Decimal,
    pub gross: Decimal,
    /// Grouped by `(category, rate)`, in first-seen order (matches the mock's `Map` insertion
    /// order — a `BTreeMap` keyed by `(category, rate)` would re-sort, so this is a `Vec` built by
    /// scanning lines in order and appending a new group only the first time a key is seen,
    /// exactly like `vatGroups.set(key, ...)` on a fresh `Map`).
    pub vat_by_category: Vec<VatCategoryTotal>,
}

/// `InvoiceDiscountInput` (`totals.ts:80`) — a percentage of `Σ L'` or a flat amount, or none.
#[derive(Debug, Clone, Copy)]
pub enum InvoiceDiscountInput {
    Pct(Decimal),
    Amount(Decimal),
}

/// `spreadProportionally` (`totals.ts:87-116`): spreads `total` over `weights` in proportion, with
/// "largest remainder" rounding so the shares sum to exactly `round2(total)`. A weight-0 line
/// (fully-discounted, or 0 qty) always gets a 0 share, never a stray cent from rounding.
fn spread_proportionally(total: Decimal, weights: &[Decimal]) -> Vec<Decimal> {
    let target = round2(total);
    let weight_sum: Decimal = weights.iter().fold(Decimal::ZERO, |a, w| a + *w);
    if target.is_zero() || weight_sum <= Decimal::ZERO {
        return weights.iter().map(|_| Decimal::ZERO).collect();
    }

    // Exact (unrounded) share per line, then round2 each — the "proportional" part.
    let raw: Vec<Decimal> = weights.iter().map(|w| (target * *w) / weight_sum).collect();
    let mut rounded: Vec<Decimal> = raw.iter().map(|r| round2(*r)).collect();
    let mut diff = round2(target - rounded.iter().fold(Decimal::ZERO, |a, r| a + *r));

    if !diff.is_zero() {
        // Distribute the leftover cents to the lines with the largest fractional remainders (or
        // smallest, when diff < 0 and a cent must be taken back), one cent at a time — this is
        // what makes `Σ hᵢ = H` exactly regardless of how the rounding fell.
        let mut remainders: Vec<(usize, Decimal)> = raw.iter().zip(rounded.iter()).enumerate().map(|(i, (r, ro))| (i, *r - *ro)).collect();
        let step = if diff > Decimal::ZERO { Decimal::new(1, 2) } else { -Decimal::new(1, 2) };
        if diff > Decimal::ZERO {
            remainders.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            remainders.sort_by(|a, b| a.1.cmp(&b.1));
        }
        let order: Vec<usize> = remainders.iter().map(|(i, _)| *i).collect();
        let mut cursor = 0usize;
        let mut guard = order.len() * 2 + 4; // rounding leftovers are at most a few cents; bounds the loop
        while diff.abs() > Decimal::new(1, 3) && guard > 0 {
            guard -= 1;
            let target_i = order[cursor % order.len()];
            if weights[target_i] > Decimal::ZERO {
                rounded[target_i] = round2(rounded[target_i] + step);
                diff = round2(diff - step);
            }
            cursor += 1;
        }
    }
    rounded
}

/// `computeInvoiceTotals` (`totals.ts:126-188`) — see this module's doc comment for the algorithm.
/// `prices_include_tax = true` for a sales invoice under `settings.prices_include_tax`;
/// `false` always for a purchase (purchase prices are never tax-inclusive, matching
/// `computePurchaseTotals`'s hard-coded `false`).
pub fn compute_invoice_totals(lines: &[TotalsLineInput], invoice_discount: Option<InvoiceDiscountInput>, prices_include_tax: bool) -> InvoiceTotalsResult {
    // Steps 1-2: per-line amount and line discount.
    struct Step2 {
        l: Decimal,
        line_discount: Decimal,
        l_prime: Decimal,
    }
    let step2: Vec<Step2> = lines
        .iter()
        .map(|line| {
            let l = round2(line.qty * line.unit_price);
            let line_discount = round2(if line.discount_is_pct { l * (line.discount / Decimal::ONE_HUNDRED) } else { line.discount });
            let l_prime = round2((l - line_discount).max(Decimal::ZERO));
            Step2 { l, line_discount, l_prime }
        })
        .collect();

    // Step 3: invoice discount H, spread proportionally over L'ᵢ with largest-remainder rounding.
    let sum_l_prime = round2(step2.iter().fold(Decimal::ZERO, |a, s| a + s.l_prime));
    let h = match invoice_discount {
        None => Decimal::ZERO,
        Some(InvoiceDiscountInput::Pct(pct)) => round2(sum_l_prime * (pct / Decimal::ONE_HUNDRED)),
        Some(InvoiceDiscountInput::Amount(amount)) => round2(amount),
    };
    let weights: Vec<Decimal> = step2.iter().map(|s| s.l_prime).collect();
    let shares = spread_proportionally(h, &weights);

    // Step 4: VAT per line (exclusive or inclusive), step 5: group by (category, rate).
    let mut group_order: Vec<(TaxCategory, Decimal)> = Vec::new();
    let mut groups: BTreeMap<(TaxCategory, Decimal), VatCategoryTotal> = BTreeMap::new();

    let results: Vec<TotalsLineResult> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let s = &step2[i];
            let invoice_discount_share = shares[i];
            let l_double_prime = round2((s.l_prime - invoice_discount_share).max(Decimal::ZERO));
            let rate = line.tax.map(|t| t.rate).unwrap_or(Decimal::ZERO);
            let category = line.tax.map(|t| t.category).unwrap_or(TaxCategory::Other);

            let (net, vat, gross) = if prices_include_tax {
                let gross = l_double_prime;
                let net = round2(gross / (Decimal::ONE + rate / Decimal::ONE_HUNDRED));
                let vat = round2(gross - net);
                (net, vat, gross)
            } else {
                let net = l_double_prime;
                let vat = round2(net * (rate / Decimal::ONE_HUNDRED));
                let gross = round2(net + vat);
                (net, vat, gross)
            };

            let key = (category, rate);
            let group = groups.entry(key).or_insert_with(|| {
                group_order.push(key);
                VatCategoryTotal { category, rate, net: Decimal::ZERO, vat: Decimal::ZERO }
            });
            group.net = round2(group.net + net);
            group.vat = round2(group.vat + vat);

            TotalsLineResult {
                l: s.l,
                line_discount: s.line_discount,
                l_prime: s.l_prime,
                invoice_discount_share,
                l_double_prime,
                net,
                vat,
                gross,
                tax_rate: rate,
                tax_category: category,
            }
        })
        .collect();

    let vat_by_category = group_order.into_iter().map(|key| groups[&key]).collect();

    InvoiceTotalsResult {
        net: round2(results.iter().fold(Decimal::ZERO, |a, r| a + r.net)),
        vat: round2(results.iter().fold(Decimal::ZERO, |a, r| a + r.vat)),
        gross: round2(results.iter().fold(Decimal::ZERO, |a, r| a + r.gross)),
        lines: results,
        sub_total_after_line_discounts: sum_l_prime,
        invoice_discount_amount: h,
        vat_by_category,
    }
}

/// `paymentStatusFor` (`totals.ts:233-237`) — kept here (rather than duplicated per domain) since
/// invoices and purchases both need the identical "paid >= total - 0.005" tolerance rule for their
/// own `PaymentStatus` enum; each domain maps this generic 3-way result onto its own DTO enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentStatusKind {
    Paid,
    PartiallyPaid,
    Unpaid,
}

pub fn payment_status_for(total: Decimal, paid: Decimal) -> PaymentStatusKind {
    if paid >= total - Decimal::new(5, 3) {
        PaymentStatusKind::Paid
    } else if paid > Decimal::ZERO {
        PaymentStatusKind::PartiallyPaid
    } else {
        PaymentStatusKind::Unpaid
    }
}

/// `invoiceOutstanding`/`purchaseOutstanding` (`totals.ts:240-242`, `purchases.ts:74-76`) — same
/// shape for both: `max(0, round2(total − adjustmentsOut − paid))`. Callers pass
/// `grand_total − refunded_amount` (invoices) or `grand_total − returned_amount` (purchase
/// orders) as `total_after_adjustments`.
pub fn document_outstanding(total_after_adjustments: Decimal, paid: Decimal) -> Decimal {
    round2(total_after_adjustments - paid).max(Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn tax(rate: i64, category: TaxCategory) -> TotalsTax {
        TotalsTax { rate: Decimal::new(rate, 0), category }
    }

    /// The worked example pinned by `totals.ts`'s own doc comment (docs/v2/06 §3, inclusive
    /// prices, line B zero-rated):
    ///   Line A: 2 × 57.50, 10% line discount, tax S 15%
    ///   Line B: 1 × 23.00, tax Z 0%
    ///   Invoice discount: 5.00 flat, spread proportionally over L' (103.50 / 23.00)
    ///   → net 108.53, VAT 12.97, gross 121.50 exactly.
    #[test]
    fn worked_example_matches_totals_ts_doc_comment() {
        let lines = [
            TotalsLineInput { qty: dec!(2), unit_price: dec!(57.50), discount: dec!(10), discount_is_pct: true, tax: Some(tax(15, TaxCategory::Standard)) },
            TotalsLineInput { qty: dec!(1), unit_price: dec!(23.00), discount: Decimal::ZERO, discount_is_pct: false, tax: Some(tax(0, TaxCategory::Zero)) },
        ];
        let result = compute_invoice_totals(&lines, Some(InvoiceDiscountInput::Amount(dec!(5.00))), true);

        assert_eq!(result.net, dec!(108.53));
        assert_eq!(result.vat, dec!(12.97));
        assert_eq!(result.gross, dec!(121.50));

        // Line A: L = 115.00, discount 10% = 11.50, L' = 103.50.
        assert_eq!(result.lines[0].l, dec!(115.00));
        assert_eq!(result.lines[0].line_discount, dec!(11.50));
        assert_eq!(result.lines[0].l_prime, dec!(103.50));
        // Line B: L = L' = 23.00.
        assert_eq!(result.lines[1].l, dec!(23.00));
        assert_eq!(result.lines[1].l_prime, dec!(23.00));

        // Invoice discount spread proportionally over (103.50, 23.00), summing to exactly 5.00.
        let share_sum = result.lines[0].invoice_discount_share + result.lines[1].invoice_discount_share;
        assert_eq!(share_sum, dec!(5.00));

        // gross Σ = 121.50 exactly; vat by category has 2 groups (S:15, Z:0).
        assert_eq!(result.vat_by_category.len(), 2);
        assert_eq!(result.vat_by_category[0].category, TaxCategory::Standard);
        assert_eq!(result.vat_by_category[1].category, TaxCategory::Zero);
    }

    #[test]
    fn no_lines_gives_all_zero_totals() {
        let result = compute_invoice_totals(&[], None, true);
        assert_eq!(result.net, Decimal::ZERO);
        assert_eq!(result.vat, Decimal::ZERO);
        assert_eq!(result.gross, Decimal::ZERO);
        assert!(result.vat_by_category.is_empty());
    }

    #[test]
    fn exclusive_prices_add_vat_on_top() {
        let lines = [TotalsLineInput { qty: Decimal::ONE, unit_price: dec!(100), discount: Decimal::ZERO, discount_is_pct: false, tax: Some(tax(15, TaxCategory::Standard)) }];
        let result = compute_invoice_totals(&lines, None, false);
        assert_eq!(result.net, dec!(100));
        assert_eq!(result.vat, dec!(15));
        assert_eq!(result.gross, dec!(115));
    }

    #[test]
    fn inclusive_prices_extract_vat_from_gross() {
        let lines = [TotalsLineInput { qty: Decimal::ONE, unit_price: dec!(115), discount: Decimal::ZERO, discount_is_pct: false, tax: Some(tax(15, TaxCategory::Standard)) }];
        let result = compute_invoice_totals(&lines, None, true);
        assert_eq!(result.gross, dec!(115));
        assert_eq!(result.net, dec!(100));
        assert_eq!(result.vat, dec!(15));
    }

    #[test]
    fn fully_discounted_line_gets_zero_invoice_discount_share() {
        let lines = [
            TotalsLineInput { qty: Decimal::ONE, unit_price: dec!(50), discount: dec!(100), discount_is_pct: true, tax: None },
            TotalsLineInput { qty: Decimal::ONE, unit_price: dec!(50), discount: Decimal::ZERO, discount_is_pct: false, tax: None },
        ];
        let result = compute_invoice_totals(&lines, Some(InvoiceDiscountInput::Amount(dec!(10))), false);
        assert_eq!(result.lines[0].l_prime, Decimal::ZERO);
        assert_eq!(result.lines[0].invoice_discount_share, Decimal::ZERO);
        assert_eq!(result.lines[1].invoice_discount_share, dec!(10));
    }

    #[test]
    fn no_tax_line_defaults_to_other_category_zero_rate() {
        let lines = [TotalsLineInput::new(Decimal::ONE, dec!(10))];
        let result = compute_invoice_totals(&lines, None, false);
        assert_eq!(result.lines[0].tax_category, TaxCategory::Other);
        assert_eq!(result.lines[0].tax_rate, Decimal::ZERO);
        assert_eq!(result.vat, Decimal::ZERO);
    }

    #[test]
    fn payment_status_matches_the_0_005_tolerance() {
        assert_eq!(payment_status_for(dec!(100), dec!(99.996)), PaymentStatusKind::Paid);
        assert_eq!(payment_status_for(dec!(100), dec!(99.995)), PaymentStatusKind::Paid);
        // 99.99 < 100 − 0.005, same as the mock's `paid >= total - 0.005` (totals.ts:255).
        assert_eq!(payment_status_for(dec!(100), dec!(99.99)), PaymentStatusKind::PartiallyPaid);
        assert_eq!(payment_status_for(dec!(100), dec!(50)), PaymentStatusKind::PartiallyPaid);
        assert_eq!(payment_status_for(dec!(100), Decimal::ZERO), PaymentStatusKind::Unpaid);
    }

    #[test]
    fn document_outstanding_never_negative() {
        assert_eq!(document_outstanding(dec!(100), dec!(120)), Decimal::ZERO);
        assert_eq!(document_outstanding(dec!(100), dec!(40)), dec!(60));
    }

    #[test]
    fn tax_category_round_trips_through_as_str() {
        for cat in [TaxCategory::Standard, TaxCategory::Zero, TaxCategory::Exempt, TaxCategory::Other] {
            assert_eq!(TaxCategory::from_str_or_other(cat.as_str()), cat);
        }
        assert_eq!(TaxCategory::from_str_or_other("bogus"), TaxCategory::Other);
    }
}

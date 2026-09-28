//! Weighted-average cost-out rules the mock keeps inline in its callers (D-2): the sale/return/
//! transfer "how much value leaves the stock for this qty" decisions. Pure functions — no DB, no
//! lock required — so a caller reads `p.stock_qty()`/`p.stock_value()`/`p.cost_price()` off an
//! already-locked [`super::LockedProduct`] and passes plain values in.

use rust_decimal::Decimal;

use crate::utils::money::round2;

/// Ports `sales.ts:340` (review A1 "sale empties the stock"): selling `qty` costs exactly
/// `stock_value` when `qty` (rounded) is at least the full remaining stock (rounded) — otherwise
/// it's the plain average-cost amount. Takes the pre-movement qty/value/cost so the caller can
/// call this before mutating the lock via [`super::apply_change`].
pub fn cost_out_sale(stock_qty: Decimal, stock_value: Decimal, cost_price: Decimal, qty: Decimal) -> Decimal {
    if round2(qty) >= round2(stock_qty) {
        stock_value
    } else {
        round2(qty * cost_price)
    }
}

/// Ports `purchases.ts:458-468` (purchase-return variance guard): the return posts at the original
/// purchase price (`qty × unit_price`), UNLESS that would leave the stock's value negative, or
/// would leave a stray non-zero value at (near-)zero qty — in which case it takes exactly what's
/// there (`stock_value`) and the rest becomes `inventoryVariance`. Returns `(value_out, variance)`;
/// `variance` is `0` on the normal path.
pub fn cost_out_at_price(stock_qty: Decimal, stock_value: Decimal, qty: Decimal, unit_price: Decimal) -> (Decimal, Decimal) {
    let at = round2(qty * unit_price);
    let new_qty = round2(stock_qty - qty);
    let new_value = round2(stock_value - at);
    let near_zero_qty = new_qty <= Decimal::new(1, 4);
    let stray_value = new_value.abs() > Decimal::new(1, 3);
    if new_value < Decimal::ZERO || (near_zero_qty && stray_value) {
        let value_out = stock_value;
        let variance = round2(at - value_out);
        (value_out, variance)
    } else {
        (at, Decimal::ZERO)
    }
}

/// Ports `transfers.ts:81-82` / `inventory.ts:159`: value moved at the company-wide weighted
/// average cost — `round2(qty * unit_cost)`.
pub fn cost_at_average(qty: Decimal, unit_cost: Decimal) -> Decimal {
    round2(qty * unit_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_out_sale_review_a1_worked_example() {
        // 9 @ 50 (value 450), receipt of 10 @ 60 -> qty 19, value 1050, cost 55.2632; sell 1.
        let stock_qty = dec!(19);
        let stock_value = dec!(1050);
        let cost_price = dec!(55.2632);
        let out = cost_out_sale(stock_qty, stock_value, cost_price, dec!(1));
        assert_eq!(out, dec!(55.26));
    }

    #[test]
    fn cost_out_sale_selling_everything_takes_exact_stock_value() {
        let out = cost_out_sale(dec!(19), dec!(1050.37), dec!(55.28), dec!(19));
        assert_eq!(out, dec!(1050.37));
        // Even a qty that rounds up to the remaining stock (19.001 -> 19.00) takes the exact value.
        let out2 = cost_out_sale(dec!(19), dec!(1050.37), dec!(55.28), dec!(19.001));
        assert_eq!(out2, dec!(1050.37));
    }

    #[test]
    fn cost_out_at_price_normal_path_no_variance() {
        let (value_out, variance) = cost_out_at_price(dec!(19), dec!(1050), dec!(5), dec!(60));
        assert_eq!(value_out, dec!(300));
        assert_eq!(variance, dec!(0));
    }

    #[test]
    fn cost_out_at_price_negative_value_guard_books_variance() {
        // Returning more value than remains: stock_value 100, qty*price = 150 -> new_value = -50 < 0.
        let (value_out, variance) = cost_out_at_price(dec!(2), dec!(100), dec!(2), dec!(75));
        assert_eq!(value_out, dec!(100));
        assert_eq!(variance, dec!(50));
    }

    #[test]
    fn cost_out_at_price_stray_value_at_zero_qty_guard() {
        // qty exactly empties stock but at_price leaves a tiny stray value at zero qty.
        let (value_out, variance) = cost_out_at_price(dec!(5), dec!(100.02), dec!(5), dec!(20));
        // at = 100.00, new_qty = 0, new_value = 0.02 -> stray (> 0.001) -> take exact stock_value.
        assert_eq!(value_out, dec!(100.02));
        assert_eq!(variance, round2(dec!(100.00) - dec!(100.02)));
    }

    #[test]
    fn cost_at_average_rounds_to_2dp() {
        assert_eq!(cost_at_average(dec!(3), dec!(55.2632)), dec!(165.79));
    }
}

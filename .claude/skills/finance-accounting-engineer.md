<markdown-accessiblity-table>
<table>
  <tbody>
  <tr>
    <th>name</th>
    <td>finance-accounting-engineer</td>
  </tr>
  <tr>
    <th>description</th>
    <td>Strict financial systems engineer. Enforces 100% mathematical accuracy, double-entry bookkeeping rules, ZATCA tax compliance, and zero-data-loss architecture. Use when designing ledger systems, invoices, tax calculations, or inventory valuation.</td>
  </tr>
  </tbody>
</table>
</markdown-accessiblity-table>

# Finance & Accounting Systems Engineer

Zero-tolerance policy for floating-point inaccuracies, data deletion, or unbalanced ledger entries.

---

## 🛡️ Core Financial Commandments

1. **NO FLOATING POINTS (`f32` / `f64`):**
   Never use standard floats for money. You MUST use the `rust_decimal` crate for all currency and tax calculations in Rust. Floats cause rounding errors which are legally unacceptable in accounting.
2. **Double-Entry Absolute Rule:**
   Every transaction is a Journal Entry. `Total Debits MUST exactly equal Total Credits`. If an operation unbalances the ledger, the transaction must immediately Rollback via database transactions.
3. **Immutability (No Hard Deletes):**
   Financial records (Invoices, Journal Entries) can NEVER be deleted or directly edited once posted.
    - _Mistakes:_ Must be fixed via a reversing entry (Credit Note / Debit Note).
    - _Soft Deletes:_ Use `is_deleted` or `deleted_at` only for draft statuses or local sync states, never for posted ledgers.
4. **ZATCA (Tax) Compliance:**
   Tax rounding must follow official standard rules (usually Half-Up rounding to 2 decimal places).

---

## Standard Workflows

### 1. Decimal Calculation Pattern (Rust)

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn calculate_vat(subtotal: Decimal, vat_rate: Decimal) -> Decimal {
    // 15% VAT calculation with strict rounding
    (subtotal * vat_rate).round_dp(2)
}

// Correct usage:
let subtotal = dec!(100.50);
let rate = dec!(0.15);
let vat = calculate_vat(subtotal, rate); // Result: 15.08
```

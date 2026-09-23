# Domain Model (Frontend Mock Data Reference)

This is the entity reference for the frontend-only rebuild. It's a **simplified** distillation of `references/accouning-system` (a real multi-tenant POS/accounting platform), scoped down to a **single store, no tenant/branch layer, no backend**. Every shape here is meant to become a TypeScript interface under `src/modules/[module]/types/` and a matching mock fixture under a `services/` mock provider — nothing here is persisted for real yet.

Where a field was dropped from the original reference, it's called out so we don't accidentally re-add SaaS/multi-tenant complexity later.

---

## 1. Account (Chart of Accounts)

```ts
interface Account {
  id: string;
  code: string;            // business key, e.g. "1110" — the ONLY identifying code (no separate internalCode)
  name: string;             // Arabic label, e.g. "الصندوق"
  groupId: string;          // ref AccountGroup
  parentId?: string;        // self-ref, for sub-accounts
  normalSide: 'DEBIT' | 'CREDIT';
  canDelete: boolean;       // false for system/default accounts
  active: boolean;
}

interface AccountGroup {
  id: string;
  code: string;
  name: string;             // "الأصول" / "الالتزامات" / "حقوق الملكية" / "الإيرادات" / "المصروفات"
  normalSide: 'DEBIT' | 'CREDIT';
  isDefault: boolean;
}
```

**Note:** the reference system's `internalCode` (random 8-char ID used as a report lookup key) was a real bug source there — deliberately not replicated. `code` is the only key.

### Default seeded Chart of Accounts (~23 accounts)

Trimmed from the reference's 43-account set down to what a single clothing/retail shop actually needs:

| Code | Name (AR) | Group | Normal Side |
|------|-----------|-------|-------------|
| 1110 | الصندوق (Cash) | Assets | Debit |
| 1120 | البنك (Bank) | Assets | Debit |
| 1130 | العملاء (Accounts Receivable) | Assets | Debit |
| 1140 | بضاعة المخزون (Inventory) | Assets | Debit |
| 1150 | ضريبة مدفوعة قابلة للاسترداد (VAT Input) | Assets | Debit |
| 2100 | الموردين (Accounts Payable) | Liabilities | Credit |
| 2150 | ضريبة مستحقة (VAT Output/Payable) | Liabilities | Credit |
| 3100 | رأس المال (Capital) | Equity | Credit |
| 3250 | أرباح محتجزة (Retained Earnings) | Equity | Credit |
| 3300 | صافي الربح/الخسارة (Net Income) | Equity | Credit |
| 3400 | مسحوبات شخصية (Drawings) | Equity | Debit |
| 4100 | المبيعات (Sales) | Revenue | Credit |
| 4200 | مرتجعات المبيعات (Sales Returns) | Revenue | Debit |
| 4300 | إيرادات أخرى (Other Income) | Revenue | Credit |
| 4400 | أرباح جرد المخزون (Stocktake Gains) | Revenue | Credit |
| 5100 | المشتريات (Purchases) | Expenses | Debit |
| 5150 | مرتجعات المشتريات (Purchase Returns) | Expenses | Credit |
| 5200 | تكلفة البضاعة المباعة (COGS) | Expenses | Debit |
| 5300 | مصروفات تشغيل (Operating Expenses) | Expenses | Debit |
| 5500 | مصروفات إدارية (Admin Expenses) | Expenses | Debit |
| 5600 | رواتب وأجور (Salaries & Wages) | Expenses | Debit |
| 5800 | خسائر جرد المخزون (Stocktake Losses) | Expenses | Debit |

Dropped entirely: Fixed Assets group (6xxx), complexity tiers, per-branch account variants.

---

## 2. Journal Entry (Double-Entry Core)

```ts
interface JournalEntry {
  id: string;
  number: string;            // auto-numbered, e.g. "JE-000123"
  date: string;               // ISO date
  description: string;
  type: 'SYSTEM' | 'MANUAL';  // SYSTEM = auto-generated from a sale/purchase/payment; MANUAL = user-entered
  sourceRef?: { kind: 'invoice' | 'purchaseOrder' | 'payment' | 'stockAdjustment'; id: string };
  lines: JournalLine[];
  totalDebit: number;
  totalCredit: number;        // must equal totalDebit before it can be "saved"
  reversed?: boolean;
  reversalOfId?: string;
  createdBy: string;          // ref User
}

interface JournalLine {
  id: string;
  accountId: string;          // ref Account
  description?: string;
  debit: number;
  credit: number;
}
```

Dropped: mandatory `costCenter` per line (cost centers cut entirely), `attachments`, polymorphic `targetRef` complexity (simplified to a plain `sourceRef` tag for UI display/linking only).

---

## 3. Product

```ts
interface Product {
  id: string;
  name: string;
  sku: string;
  barcode?: string;
  categoryId?: string;
  unitId?: string;              // "piece", "kg", etc.
  type: 'product' | 'service';  // service = no stock tracking
  costPrice: number;
  price: number;                 // base sale price
  stockQty: number;               // single number — no per-branch array
  minStock?: number;               // low-stock threshold
  active: boolean;
  image?: string;
  prices?: { priceListId: string; value: number }[]; // optional multi-price-list support
}

interface Category { id: string; name: string; }
interface Unit { id: string; name: string; }
interface PriceList { id: string; name: string; active: boolean; }
```

Dropped: per-branch `stock[]` array, `sale.account`/`purchase.account`/`sale.tax` auto-posting config (in this UI-only phase, tax/account resolution for journal preview can be hardcoded to the default Sales/COGS/VAT accounts), weight/expiry/warranty sub-schemas, custom fields.

---

## 4. Invoice (POS Sale)

```ts
interface Invoice {
  id: string;
  number: string;                // auto-numbered, e.g. "INV-000456"
  date: string;
  customerId?: string;            // undefined = walk-in sale
  cashierId: string;               // ref User
  status: 'DRAFT' | 'COMPLETED' | 'REFUNDED';
  paymentStatus: 'UNPAID' | 'PARTIALLY_PAID' | 'PAID';
  lines: InvoiceLine[];
  subTotal: number;
  discountRate: number;            // single overall %, enforced against cashier's maxDiscount
  taxAmount: number;                // aggregate VAT total
  grandTotal: number;
  paymentMethod: 'cash' | 'card' | 'bank_transfer' | 'credit';
  paidAmount: number;
  note?: string;
}

interface InvoiceLine {
  id: string;
  productId: string;
  qty: number;
  price: number;                 // snapshot at sale time
  costPrice: number;              // snapshot, used for COGS journal preview
  discount: number;                // per-line amount, optional
}
```

Dropped: per-line tax field (kept as a single aggregate, matching the reference's own approach — acceptable for a simple UI), polymorphic refund/offer refs (kept as separate simple entities below).

### Refund (Sales Return)

```ts
interface Refund {
  id: string;
  number: string;
  invoiceId: string;              // original sale
  date: string;
  reason?: string;
  lines: { invoiceLineId: string; qty: number }[];
  grandTotal: number;
}
```

---

## 5. Customer / Supplier

```ts
interface Customer {
  id: string;
  name: string;
  phone?: string;
  address?: string;
  type: 'individual' | 'company';
  vatNumber?: string;
  balance: number;                // computed display value, positive = they owe us
  active: boolean;
}

interface Supplier {
  id: string;
  name: string;
  phone?: string;
  contactPerson?: string;
  address?: string;
  vatNumber?: string;
  balance: number;                // positive = we owe them
  active: boolean;
}
```

Dropped: dedicated per-party GL sub-account (`account` ref) — `balance` is just a display number computed from mock invoice/payment data, no real ledger wiring yet.

---

## 6. Purchase Order

```ts
interface PurchaseOrder {
  id: string;
  number: string;
  supplierId: string;
  date: string;
  status: 'DRAFT' | 'CONFIRMED' | 'CANCELED';
  lines: { productId: string; qty: number; costPrice: number }[];
  subTotal: number;
  taxAmount: number;
  grandTotal: number;
  paymentStatus: 'UNPAID' | 'PARTIALLY_PAID' | 'PAID';
}
```

---

## 7. Payment

```ts
interface Payment {
  id: string;
  number: string;               // e.g. "PAY-000789"
  date: string;
  type: 'RECEIVED' | 'PAID';
  targetType: 'customer' | 'supplier';
  targetId: string;
  targetRef: string;              // e.g. linked invoice/PO id, kept simple — one payment, one target
  amount: number;
  method: 'cash' | 'card' | 'bank_transfer';
  note?: string;
}
```

Dropped: allocation engine (`allocatedAmount`/`unallocatedAmount`/`allocationStatus`, multi-invoice matching UI) — one payment maps to one invoice/PO for simplicity.

---

## 8. Inventory Adjustments

```ts
interface StockAdjustment {
  id: string;
  number: string;
  type: 'STOCK_IN' | 'LOSS' | 'STOCKTAKE';
  date: string;
  status: 'DRAFT' | 'COMPLETED';
  lines: { productId: string; systemQty?: number; countedQty?: number; qtyChange: number }[];
  note?: string;
}

interface StockMovement {              // read-only history/ledger entry
  id: string;
  date: string;
  productId: string;
  qtyChange: number;
  reason: 'sale' | 'purchase' | 'stock_in' | 'loss' | 'stocktake' | 'refund';
  refId: string;                        // invoice/PO/adjustment id
}
```

Dropped: inter-branch transfer (no branches), embedded per-branch stock arrays, undo/reversal status machine (kept simple — a correction is just a new adjustment).

---

## 9. Tax

```ts
interface Tax {
  id: string;
  name: string;                  // "ضريبة القيمة المضافة"
  rate: number;                    // percentage, e.g. 15
  type: 'OUTPUT' | 'INPUT';
  isDefault: boolean;
  active: boolean;
}
```

---

## 10. User / Role

```ts
interface User {
  id: string;
  name: string;
  phone?: string;
  role: 'admin' | 'manager' | 'accountant' | 'cashier';
  maxDiscount: number;            // % cap enforced at POS
  priceListId?: string;            // which price list this user sells at
  active: boolean;
  avatar?: string;
}
```

Role presets replace the reference's full `{module}:{action}` permission-string matrix (100+ flags). For this UI phase, gate navigation/actions by `role` alone:

| Role | Access |
|------|--------|
| admin | everything |
| manager | everything except user management |
| accountant | accounting, reports, payments (read/write); sales/purchases read-only |
| cashier | POS/sales only, read-only inventory, no accounting/reports access |

---

## 11. Fiscal Year

```ts
interface FiscalYear {
  id: string;
  name: string;                  // "2026"
  startDate: string;
  endDate: string;
  isClosed: boolean;
}
```

No period-locking enforcement logic in this phase — just a settings record the reports screens read their default date range from.

---

## 12. Settings

```ts
interface StoreSettings {
  storeName: string;
  logo?: string;
  currency: string;               // "SAR"
  vatNumber?: string;
  defaultTaxId?: string;
  invoiceNumberPrefix: string;     // e.g. "INV-"
  printer: {
    mode: 'a4' | 'thermal';
    thermalWidthMm: 58 | 80;
  };
  theme: 'light' | 'dark';
}
```

---

## What was cut, and why (see `project_specs.md` §6 for the full list)

Every dropped feature below existed in `references/accouning-system` but doesn't belong in a single-store, frontend-only tool:

- **Tenancy**: `store`, `branch`, plans/subscriptions, usage limits, super-admin app
- **Cost centers**: mandatory per-journal-line cost center tied to branches
- **Fixed assets**: full depreciation subsystem (4 methods, disposal workflow)
- **Payment allocation engine**: multi-invoice partial-payment matching
- **Real ZATCA e-invoicing**: only the cosmetic Phase-1 QR code is kept (see `references/vat-invoice-app/js/zatca-qr.js`)
- **Integrations**: WhatsApp send, push notifications, webhooks, background jobs/cron, offline sync queue
- **Chart-of-accounts complexity tiers** and the `internalCode` vs `code` split
- **Credit/debit notes**: unimplemented even in the reference — skipped

export type ProductType = 'product' | 'service';

/**
 * v2 (docs/v2/07-products-and-inventory.md §1 "أساسي"): the product-form type selector is really
 * three choices — stock item / service / non-stock item (e.g. bags). `type` stays 'product' |
 * 'service' everywhere else in the app (invoices, purchases, POS, reports all switch on it), so
 * "non-stock" is layered on top as `stockMode: 'none'` on a `type: 'product'` row: it behaves like
 * a product (has a price, appears in the catalog) but never carries stock (like a service, minus
 * the "service" labeling). `stockMode` is undefined on legacy/seed rows, treated as 'tracked'.
 */
export type StockMode = 'tracked' | 'none';

/** v2 §2 — a unit row on a product's "الوحدات والباركود" table. Exactly one row has `factor === 1` (the base unit). */
export interface ProductUnit {
  id: string;
  unitId: string;
  /** How many base units this unit contains. The base unit's own row always has factor 1. */
  factor: number;
  barcodes: string[];
  /** Sale/purchase price for this unit. Auto-computed as base price × factor until the user edits it (`priceIsAuto`). */
  price: number;
  /** True while `price` still tracks base×factor automatically ("تلقائي" badge); false once hand-edited. */
  priceIsAuto: boolean;
  defaultForSale: boolean;
  defaultForPurchase: boolean;
  /** Set once any stock movement has happened on this product — factor becomes immutable (§2 validation). Deactivate + add a new unit instead. */
  active: boolean;
}

/** v2 §1 "الأسعار" — per price-list value for a specific unit. Auto (base×factor) until edited. */
export interface ProductUnitPrice {
  priceListId: string;
  unitId: string;
  value: number;
}

/** v2 §3 — one received batch/lot of a batch-tracked product. Stock is still summed at the product level; `qty` here is this batch's remaining qty in the base unit. */
export interface ProductBatch {
  id: string;
  productId: string;
  batchNo: string;
  expiryDate?: string;
  qty: number;
  /** Unit cost at receipt (base unit), used for write-off/return valuation. */
  unitCost: number;
  supplierId?: string;
  receivedDate: string;
  /** Purchase/stock-in document this batch came from, for the expiry report's "return to supplier" action. */
  sourceRefId?: string;
  sourceRefNumber?: string;
}

/** v2 §7 (Settings → Products) — a custom field definition shown on the product form's "إضافي" tab. */
export type CustomFieldType = 'text' | 'number' | 'date' | 'list' | 'yesno';

export interface CustomFieldDef {
  id: string;
  name: string;
  type: CustomFieldType;
  /** Options for `type === 'list'`. */
  options?: string[];
  active: boolean;
  sortOrder: number;
}

export interface Product {
  id: string;
  name: string;
  nameEn?: string;
  sku: string;
  barcode?: string;
  categoryId?: string;
  unitId?: string;
  type: ProductType;
  /** v2 — 'none' = non-stock item (has a price, no stock tracking). Undefined = 'tracked' (legacy default). */
  stockMode?: StockMode;
  costPrice: number;
  price: number;
  stockQty: number;
  minStock?: number;
  active: boolean;
  image?: string;
  prices?: { priceListId: string; value: number }[];
  /** v2 (E2): service/non-stock line accounts — product override → category.purchaseAccountId → settings default. */
  purchaseAccountId?: string;
  /**
   * v2 (docs/v2/02-accounting-review.md A1/A2): the company-wide GL value of this product's stock,
   * in the base currency. `costPrice` (displayed as "متوسط التكلفة") is derived as
   * `round4(stockValue / stockQty)` and kept in sync by every stock movement — it is NOT the
   * source of truth any more. Every movement changes `stockValue` by exactly the amount posted to
   * the GL, so `GL(inventory) = Σ product.stockValue` holds exactly, not approximately.
   */
  stockValue: number;

  // --- v2 phase 6 additions (docs/v2/07-products-and-inventory.md) ---------------------------

  /** "أساسي" tab. */
  brand?: string;
  tags?: string[];
  /** Attachment ids (AttachmentField, ownerRef `product:<id>`) in display order — index 0 is the POS thumbnail. */
  imageIds?: string[];
  description?: string;

  /** "الوحدات والباركود" tab — §2. Empty/undefined = single implicit base unit (`unitId`, factor 1), same as v1. */
  units?: ProductUnit[];
  /** Per price-list × unit matrix (§1 "الأسعار"). Falls back to `prices` (base-unit-only) when empty. */
  unitPrices?: ProductUnitPrice[];
  /** Floor below which a price can't be sold (any discount/override is clamped to it). */
  minPrice?: number;

  /** "الضريبة والحسابات" tab. */
  saleTaxId?: string;
  purchaseTaxId?: string;
  revenueAccountId?: string;
  cogsAccountId?: string;

  /** "المخزون" tab. */
  allowNegativeStock?: boolean;
  shelfLocation?: string;
  preferredSupplierId?: string;
  reorderQty?: number;
  /** §3 — batch/expiry tracking toggle + alert window. */
  trackBatches?: boolean;
  expiryAlertDays?: number;

  /** "إضافي" tab. */
  warrantyMonths?: number;
  warrantyProvider?: 'manufacturer' | 'store';
  weight?: number;
  customFields?: Record<string, string | number | boolean | undefined>;
}

export type ProductInput = Omit<Product, 'id' | 'stockQty' | 'stockValue'> & {
  /** Only honored on create — becomes an opening STOCK_IN adjustment. */
  openingQty?: number;
};

export interface ProductFilter {
  search?: string;
  categoryId?: string;
  type?: ProductType;
  lowStockOnly?: boolean;
  includeInactive?: boolean;
}

export interface Category {
  id: string;
  name: string;
  /** v2 (E2): fallback purchase (expense) account for non-stock/service products in this category. */
  purchaseAccountId?: string;
  /** v2 (docs/v2/07 "Account resolution"): category-level defaults, one layer above the settings default. */
  revenueAccountId?: string;
  cogsAccountId?: string;
  saleTaxId?: string;
  purchaseTaxId?: string;
}

/** v2 §2 "Unit master" — name/symbol/allows-decimals. `Unit` (v1) is now this master row. */
export interface Unit {
  id: string;
  name: string;
  symbol?: string;
  allowsDecimals?: boolean;
}

/** v2 §2 "Presets by business type" — selectable starting sets, not enforced. */
export type UnitPresetKind = 'pharmacy' | 'clothing' | 'supermarket';

export interface PriceList {
  id: string;
  name: string;
  active: boolean;
}

export type StockAdjustmentType = 'STOCK_IN' | 'LOSS' | 'STOCKTAKE';

/**
 * v2 (docs/v2/02-accounting-review.md A3): every STOCK_IN needs a reason — it decides the credit
 * account (`accountFor()` role in parentheses):
 *   opening            → openingBalanceEquity (3900)
 *   owner_contribution → ownerCurrent (3150)
 *   gift               → otherIncome (4300) — free goods from a supplier, or a gift
 *   found              → inventoryVariance (5110, credit) — surplus found outside a stocktake
 *   other              → a user-chosen non-control account (`offsetAccountId`)
 */
export type StockInReason = 'opening' | 'owner_contribution' | 'gift' | 'found' | 'other';

export interface StockAdjustmentLine {
  productId: string;
  systemQty?: number;
  countedQty?: number;
  qtyChange: number;
  /** Extension: unit cost snapshot, used for the journal preview/value column. */
  unitCost?: number;
  /** v2 §3 — batch received on this STOCK_IN line (only for `trackBatches` products). */
  batchNo?: string;
  expiryDate?: string;
}

export interface StockAdjustment {
  id: string;
  number: string;
  type: StockAdjustmentType;
  date: string;
  status: 'DRAFT' | 'COMPLETED';
  lines: StockAdjustmentLine[];
  note?: string;
  /** STOCK_IN only (A3). */
  reason?: StockInReason;
  /** STOCK_IN with reason = 'other' only: the user-chosen credit account (must not be a control account). */
  offsetAccountId?: string;
  /** v2 (§5 approval threshold): set when this adjustment's value exceeded the configured threshold and needed a manager PIN. */
  approvedBy?: string;
  approvedAt?: string;
}

export interface StockAdjustmentInput {
  type: StockAdjustmentType;
  date: string;
  note?: string;
  reason?: StockInReason;
  offsetAccountId?: string;
  lines: { productId: string; qtyChange?: number; countedQty?: number; batchNo?: string; expiryDate?: string }[];
  approvedBy?: string;
}

export type StockMovementReason = 'sale' | 'purchase' | 'stock_in' | 'loss' | 'stocktake' | 'refund' | 'purchase_return';

export interface StockMovement {
  id: string;
  date: string;
  productId: string;
  qtyChange: number;
  /** v2 (A1/A2): the GL value this movement carried — the exact amount posted to the inventory account. */
  valueChange: number;
  reason: StockMovementReason;
  refId: string;
  /** Extension: display number of the source document (INV-…, ADJ-…). */
  refNumber?: string;
  /** Extension: stock level right after this movement — the running balance column. */
  balanceAfter?: number;
  /** v2 §3 — the batch this movement drew from/added to, for batch-tracked products. */
  batchId?: string;
}

// ---------------------------------------------------------------------------------------------
// v2 §5 — Stocktake v2 (scope, snapshot, blind count, scan counting, review)
// ---------------------------------------------------------------------------------------------

export type StockCountScope = 'all' | 'category' | 'location';
export type StockCountStatus = 'OPEN' | 'REVIEW' | 'COMPLETED';

export interface StockCountLine {
  productId: string;
  /** Snapshotted at count-start time (A5) — never re-read later. */
  systemQty: number;
  countedQty?: number;
  unitCost: number;
}

export interface StockCount {
  id: string;
  number: string;
  status: StockCountStatus;
  scope: StockCountScope;
  categoryId?: string;
  location?: string;
  blind: boolean;
  startedAt: string;
  startedBy: string;
  lines: StockCountLine[];
  note?: string;
  /** Set once applied — links to the resulting StockAdjustment (type STOCKTAKE) that posts the journal. */
  adjustmentId?: string;
}

export interface StockCountInput {
  scope: StockCountScope;
  categoryId?: string;
  location?: string;
  blind: boolean;
  note?: string;
}

// ---------------------------------------------------------------------------------------------
// v2 §4 — Expiry report actions: return-to-supplier (draft debit note, TODO phase 8) / write-off
// ---------------------------------------------------------------------------------------------

/**
 * TODO(phase 8): a minimal draft stub only — Phase 8 owns real debit-note posting
 * (docs/v2/09-purchases-payments-expenses.md "Debit notes v2"). This just records the intent
 * ("return these expired batches to this supplier") as a DRAFT so nothing is lost; it never posts
 * to the ledger and has no UI to confirm/post it beyond this list.
 */
export interface DebitNoteDraft {
  id: string;
  number: string;
  supplierId: string;
  date: string;
  status: 'DRAFT';
  lines: { productId: string; batchId: string; qty: number; unitCost: number }[];
  note?: string;
}

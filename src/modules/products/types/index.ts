export type ProductType = 'product' | 'service';

export interface Product {
  id: string;
  name: string;
  sku: string;
  barcode?: string;
  categoryId?: string;
  unitId?: string;
  type: ProductType;
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
}

export interface Unit {
  id: string;
  name: string;
}

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
}

export interface StockAdjustmentInput {
  type: StockAdjustmentType;
  date: string;
  note?: string;
  reason?: StockInReason;
  offsetAccountId?: string;
  lines: { productId: string; qtyChange?: number; countedQty?: number }[];
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
}

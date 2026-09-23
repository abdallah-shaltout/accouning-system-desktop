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
}

export type ProductInput = Omit<Product, 'id' | 'stockQty'> & {
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
}

export interface StockAdjustmentInput {
  type: StockAdjustmentType;
  date: string;
  note?: string;
  lines: { productId: string; qtyChange?: number; countedQty?: number }[];
}

export type StockMovementReason = 'sale' | 'purchase' | 'stock_in' | 'loss' | 'stocktake' | 'refund' | 'purchase_return';

export interface StockMovement {
  id: string;
  date: string;
  productId: string;
  qtyChange: number;
  reason: StockMovementReason;
  refId: string;
  /** Extension: display number of the source document (INV-…, ADJ-…). */
  refNumber?: string;
  /** Extension: stock level right after this movement — the running balance column. */
  balanceAfter?: number;
}

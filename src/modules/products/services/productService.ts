import { ApiError, clone, db, delay, includesText, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { recordStockAdjustment } from '@/mocks/backend/inventory';
import { emit } from '@/mocks/events';
import { mutate } from '@/mocks/persist';
import type { Product, ProductFilter, ProductInput } from '../types';

export function isLowStock(p: Product): boolean {
  return p.type === 'product' && p.stockQty <= (p.minStock ?? 0);
}

export async function getProducts(filter: ProductFilter = {}): Promise<Product[]> {
  await delay();
  return clone(
    db.products.filter(
      (p) =>
        (filter.includeInactive || p.active) &&
        (!filter.categoryId || p.categoryId === filter.categoryId) &&
        (!filter.type || p.type === filter.type) &&
        (!filter.lowStockOnly || isLowStock(p)) &&
        includesText([p.name, p.sku, p.barcode], filter.search),
    ),
  );
}

export async function getProduct(id: string): Promise<Product> {
  await delay();
  const product = db.products.find((p) => p.id === id);
  if (!product) throw new ApiError('المنتج غير موجود', 'NOT_FOUND');
  return clone(product);
}

/** Exact SKU/barcode lookup (barcode scanners in POS and adjustment screens). */
export async function findByCode(code: string): Promise<Product | null> {
  await delay(60);
  const c = code.trim();
  const product = db.products.find((p) => p.active && (p.barcode === c || p.sku.toLowerCase() === c.toLowerCase()));
  return product ? clone(product) : null;
}

function validate(input: ProductInput, exceptId?: string) {
  if (!input.name.trim()) throw new ApiError('اسم المنتج مطلوب');
  if (!input.sku.trim()) throw new ApiError('رمز المنتج (SKU) مطلوب');
  if (db.products.some((p) => p.id !== exceptId && p.sku.toLowerCase() === input.sku.trim().toLowerCase())) {
    throw new ApiError('رمز المنتج مستخدم لمنتج آخر', 'CONFLICT');
  }
  if (input.barcode && db.products.some((p) => p.id !== exceptId && p.barcode === input.barcode)) {
    throw new ApiError('الباركود مستخدم لمنتج آخر', 'CONFLICT');
  }
  if (input.price < 0 || input.costPrice < 0) throw new ApiError('الأسعار لا يمكن أن تكون سالبة');
}

function normalize(input: ProductInput) {
  const { openingQty: _opening, ...fields } = input;
  return {
    ...fields,
    name: fields.name.trim(),
    sku: fields.sku.trim(),
    barcode: fields.barcode?.trim() || undefined,
    categoryId: fields.categoryId || undefined,
    unitId: fields.unitId || undefined,
    minStock: fields.type === 'service' ? undefined : fields.minStock,
    costPrice: fields.type === 'service' ? fields.costPrice ?? 0 : fields.costPrice,
    prices: (fields.prices ?? []).filter((p) => p.value !== null && p.value !== undefined && !Number.isNaN(p.value)),
  };
}

export async function createProduct(input: ProductInput): Promise<Product> {
  await delay();
  validate(input);
  const product: Product = { id: uid('prd'), ...normalize(input), stockQty: 0 };
  mutate(() => db.products.push(product));
  // Opening stock goes through a real STOCK_IN adjustment so it has a movement + journal entry.
  if (product.type === 'product' && (input.openingQty ?? 0) > 0) {
    recordStockAdjustment(
      { type: 'STOCK_IN', date: new Date().toISOString(), note: `رصيد افتتاحي — ${product.name}`, lines: [{ productId: product.id, qtyChange: input.openingQty }] },
      session.userId,
    );
  }
  logActivity('product', `إضافة المنتج ${product.name}`, session.userId, new Date().toISOString(), `/products/${product.id}`);
  emit('catalog:changed');
  return clone(product);
}

export async function updateProduct(id: string, input: ProductInput): Promise<Product> {
  await delay();
  const product = db.products.find((p) => p.id === id);
  if (!product) throw new ApiError('المنتج غير موجود', 'NOT_FOUND');
  validate(input, id);
  if (input.type !== product.type && product.stockQty !== 0) {
    throw new ApiError('لا يمكن تحويل منتج له رصيد مخزون إلى خدمة — صفّر المخزون أولاً');
  }
  mutate(() => Object.assign(product, normalize(input)));
  logActivity('product', `تعديل المنتج ${product.name}`, session.userId, new Date().toISOString(), `/products/${product.id}`);
  emit('catalog:changed');
  return clone(product);
}

/** Suggest the next free SKU for a category prefix, e.g. MEN-008. */
export async function suggestSku(prefix: string): Promise<string> {
  await delay(40);
  const used = db.products
    .map((p) => p.sku)
    .filter((s) => s.startsWith(`${prefix}-`))
    .map((s) => Number(s.slice(prefix.length + 1)) || 0);
  return `${prefix}-${String(Math.max(0, ...used) + 1).padStart(3, '0')}`;
}

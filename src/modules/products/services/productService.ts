import { ApiError, clone, db, delay, includesText, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { recordStockAdjustment } from '@/mocks/backend/inventory';
import { emit } from '@/mocks/events';
import { mutate } from '@/mocks/persist';
import type { Product, ProductFilter, ProductInput, ProductUnit } from '../types';

export function isLowStock(p: Product): boolean {
  return p.type === 'product' && p.stockMode !== 'none' && p.stockQty <= (p.minStock ?? 0);
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

/**
 * v2 §2 validation (docs/v2/07-products-and-inventory.md): factors are positive (decimals OK for
 * weight units), exactly one unit has factor 1, and — once stock has moved — no existing unit's
 * factor may change (add a new unit + deactivate the old one instead).
 */
function validateUnits(units: ProductUnit[] | undefined, existing: Product | undefined) {
  if (!units?.length) return;
  for (const u of units) {
    if (!(u.factor > 0)) throw new ApiError('عامل تحويل الوحدة يجب أن يكون أكبر من صفر');
  }
  const baseCount = units.filter((u) => u.factor === 1).length;
  if (baseCount !== 1) throw new ApiError('يجب أن تكون وحدة واحدة فقط بعامل تحويل = 1 (الوحدة الأساسية)');
  if (existing?.stockQty && existing.stockQty > 0.0001 && existing.units?.length) {
    for (const oldUnit of existing.units) {
      const stillThere = units.find((u) => u.id === oldUnit.id);
      if (stillThere && stillThere.factor !== oldUnit.factor) {
        throw new ApiError(`لا يمكن تغيير عامل تحويل وحدة "${oldUnit.unitId}" بعد تحرك المخزون — أضف وحدة جديدة وعطّل القديمة بدلاً من ذلك`);
      }
    }
  }
  // Barcodes must be unique across all of a product's own units too, not just across products.
  const allBarcodes = units.flatMap((u) => u.barcodes.filter(Boolean));
  if (new Set(allBarcodes).size !== allBarcodes.length) throw new ApiError('نفس الباركود مستخدم أكثر من مرة في وحدات هذا المنتج');
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
  const otherBarcodes = new Set(
    db.products.filter((p) => p.id !== exceptId).flatMap((p) => (p.units ?? []).flatMap((u) => u.barcodes)),
  );
  for (const u of input.units ?? []) {
    for (const bc of u.barcodes) {
      if (bc && otherBarcodes.has(bc)) throw new ApiError(`الباركود "${bc}" مستخدم في منتج آخر`, 'CONFLICT');
    }
  }
  if (input.price < 0 || input.costPrice < 0) throw new ApiError('الأسعار لا يمكن أن تكون سالبة');
  if (input.minPrice !== undefined && input.minPrice > input.price) throw new ApiError('الحد الأدنى للسعر أكبر من سعر البيع');
  validateUnits(input.units, exceptId ? db.products.find((p) => p.id === exceptId) : undefined);
}

function normalize(input: ProductInput) {
  const { openingQty: _opening, ...fields } = input;
  return {
    ...fields,
    name: fields.name.trim(),
    nameEn: fields.nameEn?.trim() || undefined,
    sku: fields.sku.trim(),
    barcode: fields.barcode?.trim() || undefined,
    categoryId: fields.categoryId || undefined,
    unitId: fields.unitId || undefined,
    minStock: fields.type === 'service' ? undefined : fields.minStock,
    costPrice: fields.type === 'service' ? fields.costPrice ?? 0 : fields.costPrice,
    prices: (fields.prices ?? []).filter((p) => p.value !== null && p.value !== undefined && !Number.isNaN(p.value)),
    units: fields.units?.map((u) => ({ ...u, barcodes: u.barcodes.filter(Boolean) })),
    tags: fields.tags?.filter(Boolean),
  };
}

/** §2 "Generate EAN-13" — internal prefix 628 (Saudi GS1) + a random body + a valid check digit. */
export async function generateEan13(): Promise<string> {
  await delay(30);
  for (let attempt = 0; attempt < 20; attempt++) {
    const body = `628${String(Math.floor(Math.random() * 1e9)).padStart(9, '0')}`;
    const digits = body.split('').map(Number);
    const total = digits.reduce((acc, d, i) => acc + d * (i % 2 === 0 ? 1 : 3), 0);
    const check = (10 - (total % 10)) % 10;
    const code = body + check;
    const used = db.products.some((p) => p.barcode === code || (p.units ?? []).some((u) => u.barcodes.includes(code)));
    if (!used) return code;
  }
  throw new ApiError('تعذر توليد باركود فريد — حاول مرة أخرى');
}

export async function createProduct(input: ProductInput): Promise<Product> {
  await delay();
  validate(input);
  const product: Product = { id: uid('prd'), ...normalize(input), stockQty: 0, stockValue: 0 };
  mutate(() => db.products.push(product));
  // Opening stock goes through a real STOCK_IN adjustment so it has a movement + journal entry
  // (A3: reason = opening → credits openingBalanceEquity, not capital).
  if (product.type === 'product' && product.stockMode !== 'none' && (input.openingQty ?? 0) > 0) {
    recordStockAdjustment(
      { type: 'STOCK_IN', date: new Date().toISOString(), note: `رصيد افتتاحي — ${product.name}`, reason: 'opening', lines: [{ productId: product.id, qtyChange: input.openingQty }] },
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
  mutate(() => {
    const { costPrice: _formCostPrice, ...rest } = normalize(input);
    Object.assign(product, rest);
    // A1/A2: costPrice is derived from stockValue/stockQty once there's stock on hand — a stock
    // movement is the only thing allowed to change it after that. Only a still-empty product can
    // have its starting cost edited directly from the form.
    if (product.stockQty <= 0.0001) product.costPrice = _formCostPrice;
  });
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

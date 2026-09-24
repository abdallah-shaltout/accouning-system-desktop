/**
 * v2 phase 5 (docs/v2/05-onboarding.md §5): import descriptors for every master-data type the
 * wizard/checklist and the customer list wire the generic `ImportWizard` into. Each descriptor's
 * `commit()` calls the SAME module services the manual forms use (`saveCustomer`, `saveSupplier`,
 * `createProduct`…), so an imported row goes through the exact same validation/posting path a
 * hand-typed one would.
 */
import { getCustomers, saveCustomer, saveSupplier } from '@/modules/parties/services/partyService';
import { getProducts, createProduct } from '@/modules/products/services/productService';
import { getCategories, getUnits } from '@/modules/products/services/catalogService';
import type { ImportCommitSummary, ImportDescriptor, ImportRowResult } from './types';

function str(raw: unknown): string | undefined {
  const s = raw === undefined || raw === null ? '' : String(raw).trim();
  return s || undefined;
}
function numOrUndefined(raw: unknown): number | undefined {
  if (raw === undefined || raw === null || raw === '') return undefined;
  const n = Number(raw);
  return Number.isFinite(n) ? n : undefined;
}

/** Runs `commit` over rows in fixed-size batches, reporting progress — every descriptor below shares this shape. */
async function runBatched<Row>(
  rows: ImportRowResult<Row>[],
  onProgress: ((done: number, total: number) => void) | undefined,
  handleOne: (row: ImportRowResult<Row>) => Promise<'created' | 'updated' | 'skipped'>,
): Promise<ImportCommitSummary> {
  const summary: ImportCommitSummary = { created: 0, updated: 0, skipped: 0, failed: [] };
  const batchSize = 25;
  for (let i = 0; i < rows.length; i += batchSize) {
    const batch = rows.slice(i, i + batchSize);
    for (const row of batch) {
      try {
        const outcome = await handleOne(row);
        summary[outcome] += 1;
      } catch (err) {
        summary.failed.push({ index: row.index, reason: err instanceof Error ? err.message : String(err) });
      }
    }
    onProgress?.(Math.min(i + batchSize, rows.length), rows.length);
  }
  return summary;
}

// ---------------------------------------------------------------------------------------------
// Customers
// ---------------------------------------------------------------------------------------------

interface CustomerImportRow {
  name: string;
  phone?: string;
  email?: string;
  openingAmount?: number;
  openingSide?: 'debit' | 'credit';
}

export const customersDescriptor: ImportDescriptor<CustomerImportRow> = {
  key: 'customers',
  title: 'العملاء',
  entityLabel: 'عميل',
  dedupeKey: 'phone',
  columns: [
    { key: 'name', label: 'الاسم', synonyms: ['name', 'اسم العميل'], required: true, sample: 'محمد أحمد', parse: str },
    { key: 'phone', label: 'الهاتف', synonyms: ['phone', 'mobile', 'رقم الهاتف', 'الجوال'], sample: '0501234567', parse: str },
    { key: 'email', label: 'البريد الإلكتروني', synonyms: ['email', 'البريد'], parse: str },
    { key: 'openingAmount', label: 'الرصيد الافتتاحي', synonyms: ['opening balance', 'الرصيد'], note: 'اختياري — رقم فقط', sample: 0, parse: numOrUndefined },
    {
      key: 'openingSide',
      label: 'الجهة',
      synonyms: ['side'],
      note: 'مدين أو دائن',
      options: ['مدين', 'دائن'],
      parse: (raw) => (str(raw) === 'دائن' ? 'credit' : 'debit'),
      display: (v) => (v === 'credit' ? 'دائن' : 'مدين'),
    },
  ],
  sampleRows: [{ الاسم: 'محمد أحمد', الهاتف: '0501234567', 'البريد الإلكتروني': '', 'الرصيد الافتتاحي': 0, الجهة: 'مدين' }],
  async commit(rows, onProgress) {
    const existing = await getCustomers({ includeInactive: true });
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      const dup = r.row.phone ? existing.find((c) => c.phone === r.row.phone) : undefined;
      if (dup) {
        createdRows.push({ id: dup.id, name: dup.name, openingAmount: r.row.openingAmount, openingSide: r.row.openingSide });
        return 'skipped';
      }
      const saved = await saveCustomer({ type: 'individual', name: r.row.name!, phone: r.row.phone, email: r.row.email, active: true } as any);
      existing.push(saved as any);
      createdRows.push({ id: saved.id, name: saved.name, openingAmount: r.row.openingAmount, openingSide: r.row.openingSide });
      return 'created';
    });
    return { ...summary, createdRows };
  },
};

// ---------------------------------------------------------------------------------------------
// Suppliers
// ---------------------------------------------------------------------------------------------

interface SupplierImportRow {
  name: string;
  phone?: string;
  contactPerson?: string;
}

export const suppliersDescriptor: ImportDescriptor<SupplierImportRow> = {
  key: 'suppliers',
  title: 'الموردين',
  entityLabel: 'مورد',
  dedupeKey: 'phone',
  columns: [
    { key: 'name', label: 'الاسم', synonyms: ['name', 'اسم المورد'], required: true, sample: 'مصنع الأمل', parse: str },
    { key: 'phone', label: 'الهاتف', synonyms: ['phone', 'رقم الهاتف'], sample: '0555555555', parse: str },
    { key: 'contactPerson', label: 'الشخص المسؤول', synonyms: ['contact person', 'المسؤول'], parse: str },
  ],
  sampleRows: [{ الاسم: 'مصنع الأمل', الهاتف: '0555555555', 'الشخص المسؤول': '' }],
  async commit(rows, onProgress) {
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      const saved = await saveSupplier({ type: 'company', name: r.row.name!, phone: r.row.phone, contactPerson: r.row.contactPerson, active: true } as any);
      createdRows.push({ id: saved.id, name: saved.name });
      return 'created';
    });
    return { ...summary, createdRows };
  },
};

// ---------------------------------------------------------------------------------------------
// Products
// ---------------------------------------------------------------------------------------------

interface ProductImportRow {
  name: string;
  sku?: string;
  barcode?: string;
  categoryName?: string;
  unitName?: string;
  price: number;
  costPrice?: number;
  openingQty?: number;
}

export const productsDescriptor: ImportDescriptor<ProductImportRow> = {
  key: 'products',
  title: 'المنتجات',
  entityLabel: 'منتج',
  dedupeKey: 'sku',
  columns: [
    { key: 'name', label: 'اسم المنتج', synonyms: ['name', 'product name'], required: true, sample: 'قميص قطن', parse: str },
    { key: 'sku', label: 'رمز الصنف (SKU)', synonyms: ['sku', 'code'], parse: str },
    { key: 'barcode', label: 'الباركود', synonyms: ['barcode'], parse: str },
    { key: 'categoryName', label: 'التصنيف', synonyms: ['category'], parse: str },
    { key: 'unitName', label: 'الوحدة', synonyms: ['unit'], parse: str },
    { key: 'price', label: 'سعر البيع', synonyms: ['price', 'sale price'], required: true, sample: 50, parse: numOrUndefined },
    { key: 'costPrice', label: 'سعر التكلفة', synonyms: ['cost', 'cost price'], sample: 30, parse: numOrUndefined },
    { key: 'openingQty', label: 'الكمية الافتتاحية', synonyms: ['opening qty', 'الكمية'], sample: 0, parse: numOrUndefined },
  ],
  sampleRows: [{ 'اسم المنتج': 'قميص قطن', 'رمز الصنف (SKU)': 'SKU-001', الباركود: '', التصنيف: '', الوحدة: '', 'سعر البيع': 50, 'سعر التكلفة': 30, 'الكمية الافتتاحية': 0 }],
  async commit(rows, onProgress) {
    const [existing, categories, units] = await Promise.all([getProducts({ includeInactive: true }), getCategories(), getUnits()]);
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      const dup = r.row.sku ? existing.find((p) => p.sku?.toLowerCase() === r.row.sku!.toLowerCase()) : undefined;
      if (dup) {
        createdRows.push({ id: dup.id, name: dup.name });
        return 'skipped';
      }
      const category = categories.find((c) => c.name === r.row.categoryName);
      const unit = units.find((u) => u.name === r.row.unitName) ?? units[0];
      const saved = await createProduct({
        name: r.row.name!,
        sku: r.row.sku || `SKU-${Date.now()}-${r.index}`,
        barcode: r.row.barcode,
        categoryId: category?.id,
        unitId: unit?.id,
        type: 'product',
        costPrice: r.row.costPrice ?? 0,
        price: r.row.price,
        active: true,
        openingQty: r.row.openingQty,
      } as any);
      existing.push(saved);
      createdRows.push({ id: saved.id, name: saved.name });
      return 'created';
    });
    return { ...summary, createdRows };
  },
};

// ---------------------------------------------------------------------------------------------
// Opening stock (docs/v2/05 §3 tab 4) — resolves/creates products by SKU, returns rows with real
// productId for the opening-balances step's stock tab to post via `postOpeningStockForBranch`.
// ---------------------------------------------------------------------------------------------

interface OpeningStockImportRow {
  sku: string;
  qty: number;
  unitCost?: number;
  batchNo?: string;
  expiryDate?: string;
}

export const openingStockDescriptor: ImportDescriptor<OpeningStockImportRow> = {
  key: 'openingStock',
  title: 'المخزون الافتتاحي',
  entityLabel: 'صنف',
  dedupeKey: 'sku',
  columns: [
    { key: 'sku', label: 'رمز الصنف (SKU)', synonyms: ['sku', 'code', 'الباركود'], required: true, sample: 'SKU-001', parse: str },
    { key: 'qty', label: 'الكمية', synonyms: ['qty', 'quantity'], required: true, sample: 10, parse: numOrUndefined },
    { key: 'unitCost', label: 'تكلفة الوحدة', synonyms: ['unit cost', 'cost'], sample: 30, parse: numOrUndefined },
    { key: 'batchNo', label: 'رقم التشغيلة', synonyms: ['batch'], parse: str },
    { key: 'expiryDate', label: 'تاريخ الانتهاء', synonyms: ['expiry'], parse: str },
  ],
  sampleRows: [{ 'رمز الصنف (SKU)': 'SKU-001', الكمية: 10, 'تكلفة الوحدة': 30, 'رقم التشغيلة': '', 'تاريخ الانتهاء': '' }],
  async commit(rows, onProgress) {
    const products = await getProducts({ includeInactive: true });
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      const product = products.find((p) => p.sku?.toLowerCase() === r.row.sku?.toLowerCase() || p.barcode === r.row.sku);
      if (!product) {
        throw new Error(`الصنف "${r.row.sku}" غير موجود في الكتالوج — أضفه أولاً من استيراد المنتجات`);
      }
      createdRows.push({ productId: product.id, name: product.name, qty: r.row.qty, unitCost: r.row.unitCost ?? product.costPrice, batchNo: r.row.batchNo, expiryDate: r.row.expiryDate });
      return 'created';
    });
    return { ...summary, createdRows };
  },
};

// ---------------------------------------------------------------------------------------------
// Opening balances (other parties/accounts) — a simplified sheet feeding the "other" tab.
// ---------------------------------------------------------------------------------------------

interface OpeningBalanceImportRow {
  accountName: string;
  side: 'debit' | 'credit';
  amount: number;
  description?: string;
}

export const openingBalancesDescriptor: ImportDescriptor<OpeningBalanceImportRow> = {
  key: 'openingBalances',
  title: 'الأرصدة الافتتاحية (أخرى)',
  entityLabel: 'رصيد',
  columns: [
    { key: 'accountName', label: 'اسم الحساب', synonyms: ['account'], required: true, parse: str },
    {
      key: 'side',
      label: 'الجهة',
      synonyms: ['side'],
      required: true,
      options: ['مدين', 'دائن'],
      parse: (raw) => (str(raw) === 'دائن' ? 'credit' : 'debit'),
      display: (v) => (v === 'credit' ? 'دائن' : 'مدين'),
    },
    { key: 'amount', label: 'المبلغ', synonyms: ['amount'], required: true, parse: numOrUndefined },
    { key: 'description', label: 'بيان', synonyms: ['description', 'notes'], parse: str },
  ],
  sampleRows: [{ 'اسم الحساب': 'أثاث وديكورات', الجهة: 'مدين', المبلغ: 15000, بيان: 'أثاث المحل' }],
  async commit(rows, onProgress) {
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      createdRows.push({ accountName: r.row.accountName, side: r.row.side, amount: r.row.amount, description: r.row.description });
      return 'created';
    });
    return { ...summary, createdRows };
  },
};

// ---------------------------------------------------------------------------------------------
// Price update
// ---------------------------------------------------------------------------------------------

interface PriceUpdateImportRow {
  sku: string;
  price: number;
}

export const priceUpdateDescriptor: ImportDescriptor<PriceUpdateImportRow> = {
  key: 'priceUpdate',
  title: 'تحديث الأسعار',
  entityLabel: 'صنف',
  dedupeKey: 'sku',
  columns: [
    { key: 'sku', label: 'رمز الصنف (SKU)', synonyms: ['sku', 'code'], required: true, parse: str },
    { key: 'price', label: 'السعر الجديد', synonyms: ['price', 'new price'], required: true, parse: numOrUndefined },
  ],
  sampleRows: [{ 'رمز الصنف (SKU)': 'SKU-001', 'السعر الجديد': 55 }],
  async commit(rows, onProgress) {
    const { updateProduct } = await import('@/modules/products/services/productService');
    const products = await getProducts({ includeInactive: true });
    const createdRows: any[] = [];
    const summary = await runBatched(rows, onProgress, async (r) => {
      const product = products.find((p) => p.sku?.toLowerCase() === r.row.sku?.toLowerCase());
      if (!product) throw new Error(`الصنف "${r.row.sku}" غير موجود`);
      await updateProduct(product.id, { ...product, price: r.row.price } as any);
      createdRows.push({ id: product.id, name: product.name });
      return 'updated';
    });
    return { ...summary, createdRows };
  },
};

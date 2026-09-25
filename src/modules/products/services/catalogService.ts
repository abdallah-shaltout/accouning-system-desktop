import { ApiError, clone, db, delay, uid } from '@/mocks';
import { emit } from '@/mocks/events';
import { mutate } from '@/mocks/persist';
import type { Category, CustomFieldDef, PriceList, Unit, UnitPresetKind } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

type Named = { id: string; name: string };

function assertName(list: Named[], name: string, exceptId?: string) {
  const trimmed = name.trim();
  if (!trimmed) throw new ApiError('الاسم مطلوب');
  if (list.some((x) => x.id !== exceptId && x.name.trim() === trimmed)) throw new ApiError('الاسم مستخدم من قبل', 'CONFLICT');
  return trimmed;
}

// --- Categories -------------------------------------------------------------------------------

export const getCategories = wrap('products.getCategories', async function getCategories(): Promise<(Category & { productCount: number })[]> {
  await delay(120);
  return db.categories.map((c) => ({ ...clone(c), productCount: db.products.filter((p) => p.categoryId === c.id).length }));
});

export const saveCategory = wrap('products.saveCategory', async function saveCategory(name: string, id?: string, defaults?: Partial<Pick<Category, 'purchaseAccountId' | 'revenueAccountId' | 'cogsAccountId' | 'saleTaxId' | 'purchaseTaxId'>>): Promise<Category> {
  await delay();
  const clean = assertName(db.categories, name, id);
  if (id) {
    const found = db.categories.find((c) => c.id === id);
    if (!found) throw new ApiError('التصنيف غير موجود', 'NOT_FOUND');
    mutate(() => Object.assign(found, { name: clean, ...defaults }));
    emit('catalog:changed');
    return clone(found);
  }
  const category = { id: uid('cat'), name: clean, ...defaults };
  mutate(() => db.categories.push(category));
  emit('catalog:changed');
  return clone(category);
});

export const deleteCategory = wrap('products.deleteCategory', async function deleteCategory(id: string): Promise<void> {
  await delay();
  if (db.products.some((p) => p.categoryId === id)) throw new ApiError('لا يمكن حذف تصنيف مرتبط بمنتجات', 'CONFLICT');
  mutate(() => (db.categories = db.categories.filter((c) => c.id !== id)));
  emit('catalog:changed');
});

// --- Units ------------------------------------------------------------------------------------

export const getUnits = wrap('products.getUnits', async function getUnits(): Promise<(Unit & { productCount: number })[]> {
  await delay(120);
  return db.units.map((u) => ({ ...clone(u), productCount: db.products.filter((p) => p.unitId === u.id).length }));
});

export const saveUnit = wrap('products.saveUnit', async function saveUnit(name: string, id?: string, extra?: Partial<Pick<Unit, 'symbol' | 'allowsDecimals'>>): Promise<Unit> {
  await delay();
  const clean = assertName(db.units, name, id);
  if (id) {
    const found = db.units.find((u) => u.id === id);
    if (!found) throw new ApiError('الوحدة غير موجودة', 'NOT_FOUND');
    mutate(() => Object.assign(found, { name: clean, ...extra }));
    emit('catalog:changed');
    return clone(found);
  }
  const unit = { id: uid('unit'), name: clean, ...extra };
  mutate(() => db.units.push(unit));
  emit('catalog:changed');
  return clone(unit);
});

/** §2 "Presets by business type" — selectable starting sets of units, not enforced. Creates any that don't already exist by name. */
const UNIT_PRESETS: Record<UnitPresetKind, { name: string; symbol: string; allowsDecimals?: boolean }[]> = {
  pharmacy: [
    { name: 'علبة', symbol: 'box' },
    { name: 'شريط', symbol: 'strip' },
    { name: 'قرص', symbol: 'tab' },
    { name: 'زجاجة', symbol: 'btl' },
    { name: 'أمبول', symbol: 'amp' },
  ],
  clothing: [
    { name: 'قطعة', symbol: 'pc' },
    { name: 'طقم', symbol: 'set' },
    { name: 'درزن', symbol: 'dz' },
  ],
  supermarket: [
    { name: 'حبة', symbol: 'pc' },
    { name: 'كرتون', symbol: 'ctn' },
    { name: 'كيلو', symbol: 'kg', allowsDecimals: true },
    { name: 'جرام', symbol: 'g', allowsDecimals: true },
    { name: 'لتر', symbol: 'l', allowsDecimals: true },
  ],
};

export const applyUnitPreset = wrap('products.applyUnitPreset', async function applyUnitPreset(kind: UnitPresetKind): Promise<Unit[]> {
  await delay();
  const existingNames = new Set(db.units.map((u) => u.name));
  const created: Unit[] = [];
  mutate(() => {
    for (const preset of UNIT_PRESETS[kind]) {
      if (existingNames.has(preset.name)) continue;
      const unit: Unit = { id: uid('unit'), ...preset };
      db.units.push(unit);
      created.push(unit);
    }
  });
  if (created.length) emit('catalog:changed');
  return clone(created);
});

export const deleteUnit = wrap('products.deleteUnit', async function deleteUnit(id: string): Promise<void> {
  await delay();
  if (db.products.some((p) => p.unitId === id)) throw new ApiError('لا يمكن حذف وحدة مرتبطة بمنتجات', 'CONFLICT');
  mutate(() => (db.units = db.units.filter((u) => u.id !== id)));
  emit('catalog:changed');
});

// --- Price lists ------------------------------------------------------------------------------

export const getPriceLists = wrap('products.getPriceLists', async function getPriceLists(): Promise<PriceList[]> {
  await delay(120);
  return clone(db.priceLists);
});

export const savePriceList = wrap('products.savePriceList', async function savePriceList(input: { name: string; active: boolean }, id?: string): Promise<PriceList> {
  await delay();
  const name = assertName(db.priceLists, input.name, id);
  if (id) {
    const found = db.priceLists.find((p) => p.id === id);
    if (!found) throw new ApiError('قائمة الأسعار غير موجودة', 'NOT_FOUND');
    mutate(() => Object.assign(found, { name, active: input.active }));
    emit('catalog:changed');
    return clone(found);
  }
  const list = { id: uid('pl'), name, active: input.active };
  mutate(() => db.priceLists.push(list));
  emit('catalog:changed');
  return clone(list);
});

export const deletePriceList = wrap('products.deletePriceList', async function deletePriceList(id: string): Promise<void> {
  await delay();
  if (db.users.some((u) => u.priceListId === id)) throw new ApiError('قائمة الأسعار مسندة لمستخدمين — أزل الإسناد أولاً', 'CONFLICT');
  mutate(() => {
    db.priceLists = db.priceLists.filter((p) => p.id !== id);
    for (const product of db.products) product.prices = product.prices?.filter((x) => x.priceListId !== id);
  });
  emit('catalog:changed');
});

/** Bulk-update one price list's values: `{ productId: price | null }` (null removes the override). */
export const setPriceListValues = wrap('products.setPriceListValues', async function setPriceListValues(priceListId: string, values: Record<string, number | null>): Promise<void> {
  await delay();
  if (!db.priceLists.some((p) => p.id === priceListId)) throw new ApiError('قائمة الأسعار غير موجودة', 'NOT_FOUND');
  mutate(() => {
    for (const [productId, value] of Object.entries(values)) {
      const product = db.products.find((p) => p.id === productId);
      if (!product) continue;
      const others = (product.prices ?? []).filter((x) => x.priceListId !== priceListId);
      if (value === null || Number.isNaN(value)) product.prices = others;
      else {
        if (value < 0) throw new ApiError(`سعر "${product.name}" لا يمكن أن يكون سالباً`);
        product.prices = [...others, { priceListId, value }];
      }
    }
  });
  emit('catalog:changed');
});

// ---------------------------------------------------------------------------------------------
// v2 phase 6 §7 (Settings → Products) — custom field definitions for the product form's "إضافي" tab.
// ---------------------------------------------------------------------------------------------

export const getCustomFieldDefs = wrap('products.getCustomFieldDefs', async function getCustomFieldDefs(): Promise<CustomFieldDef[]> {
  await delay(100);
  return clone([...db.customFieldDefs].sort((a, b) => a.sortOrder - b.sortOrder));
});

export type CustomFieldDefInput = Omit<CustomFieldDef, 'id' | 'sortOrder'>;

export const saveCustomFieldDef = wrap('products.saveCustomFieldDef', async function saveCustomFieldDef(input: CustomFieldDefInput, id?: string): Promise<CustomFieldDef> {
  await delay();
  if (!input.name.trim()) throw new ApiError('اسم الحقل مطلوب');
  if (input.type === 'list' && !(input.options?.length)) throw new ApiError('أضف خيارات لحقل من نوع قائمة');
  let def: CustomFieldDef;
  mutate(() => {
    if (id) {
      const found = db.customFieldDefs.find((f) => f.id === id);
      if (!found) throw new ApiError('الحقل غير موجود', 'NOT_FOUND');
      Object.assign(found, input, { name: input.name.trim() });
      def = found;
    } else {
      def = { id: uid('cf'), sortOrder: db.customFieldDefs.length + 1, ...input, name: input.name.trim() };
      db.customFieldDefs.push(def);
    }
  });
  emit('catalog:changed');
  return clone(def!);
});

export const deleteCustomFieldDef = wrap('products.deleteCustomFieldDef', async function deleteCustomFieldDef(id: string): Promise<void> {
  await delay();
  mutate(() => (db.customFieldDefs = db.customFieldDefs.filter((f) => f.id !== id)));
  emit('catalog:changed');
});

export const reorderCustomFieldDefs = wrap('products.reorderCustomFieldDefs', async function reorderCustomFieldDefs(orderedIds: string[]): Promise<void> {
  await delay(60);
  mutate(() => orderedIds.forEach((id, i) => {
    const def = db.customFieldDefs.find((f) => f.id === id);
    if (def) def.sortOrder = i + 1;
  }));
  emit('catalog:changed');
});

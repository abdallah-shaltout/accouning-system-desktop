import { ApiError, clone, db, delay, uid } from '@/mocks';
import { emit } from '@/mocks/events';
import { mutate } from '@/mocks/persist';
import type { Category, PriceList, Unit } from '../types';

type Named = { id: string; name: string };

function assertName(list: Named[], name: string, exceptId?: string) {
  const trimmed = name.trim();
  if (!trimmed) throw new ApiError('الاسم مطلوب');
  if (list.some((x) => x.id !== exceptId && x.name.trim() === trimmed)) throw new ApiError('الاسم مستخدم من قبل', 'CONFLICT');
  return trimmed;
}

// --- Categories -------------------------------------------------------------------------------

export async function getCategories(): Promise<(Category & { productCount: number })[]> {
  await delay(120);
  return db.categories.map((c) => ({ ...clone(c), productCount: db.products.filter((p) => p.categoryId === c.id).length }));
}

export async function saveCategory(name: string, id?: string): Promise<Category> {
  await delay();
  const clean = assertName(db.categories, name, id);
  if (id) {
    const found = db.categories.find((c) => c.id === id);
    if (!found) throw new ApiError('التصنيف غير موجود', 'NOT_FOUND');
    mutate(() => (found.name = clean));
    emit('catalog:changed');
    return clone(found);
  }
  const category = { id: uid('cat'), name: clean };
  mutate(() => db.categories.push(category));
  emit('catalog:changed');
  return clone(category);
}

export async function deleteCategory(id: string): Promise<void> {
  await delay();
  if (db.products.some((p) => p.categoryId === id)) throw new ApiError('لا يمكن حذف تصنيف مرتبط بمنتجات', 'CONFLICT');
  mutate(() => (db.categories = db.categories.filter((c) => c.id !== id)));
  emit('catalog:changed');
}

// --- Units ------------------------------------------------------------------------------------

export async function getUnits(): Promise<(Unit & { productCount: number })[]> {
  await delay(120);
  return db.units.map((u) => ({ ...clone(u), productCount: db.products.filter((p) => p.unitId === u.id).length }));
}

export async function saveUnit(name: string, id?: string): Promise<Unit> {
  await delay();
  const clean = assertName(db.units, name, id);
  if (id) {
    const found = db.units.find((u) => u.id === id);
    if (!found) throw new ApiError('الوحدة غير موجودة', 'NOT_FOUND');
    mutate(() => (found.name = clean));
    emit('catalog:changed');
    return clone(found);
  }
  const unit = { id: uid('unit'), name: clean };
  mutate(() => db.units.push(unit));
  emit('catalog:changed');
  return clone(unit);
}

export async function deleteUnit(id: string): Promise<void> {
  await delay();
  if (db.products.some((p) => p.unitId === id)) throw new ApiError('لا يمكن حذف وحدة مرتبطة بمنتجات', 'CONFLICT');
  mutate(() => (db.units = db.units.filter((u) => u.id !== id)));
  emit('catalog:changed');
}

// --- Price lists ------------------------------------------------------------------------------

export async function getPriceLists(): Promise<PriceList[]> {
  await delay(120);
  return clone(db.priceLists);
}

export async function savePriceList(input: { name: string; active: boolean }, id?: string): Promise<PriceList> {
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
}

export async function deletePriceList(id: string): Promise<void> {
  await delay();
  if (db.users.some((u) => u.priceListId === id)) throw new ApiError('قائمة الأسعار مسندة لمستخدمين — أزل الإسناد أولاً', 'CONFLICT');
  mutate(() => {
    db.priceLists = db.priceLists.filter((p) => p.id !== id);
    for (const product of db.products) product.prices = product.prices?.filter((x) => x.priceListId !== id);
  });
  emit('catalog:changed');
}

/** Bulk-update one price list's values: `{ productId: price | null }` (null removes the override). */
export async function setPriceListValues(priceListId: string, values: Record<string, number | null>): Promise<void> {
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
}

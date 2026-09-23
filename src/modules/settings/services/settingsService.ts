import { ApiError, clone, db, delay, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { mutate } from '@/mocks/persist';
import type { StoreSettings, Tax } from '../types';

export async function getSettings(): Promise<StoreSettings> {
  await delay(80);
  return clone(db.settings);
}

export async function updateSettings(patch: Partial<StoreSettings>): Promise<StoreSettings> {
  await delay();
  if (patch.storeName !== undefined && !patch.storeName.trim()) throw new ApiError('اسم المتجر مطلوب');
  if (patch.vatNumber && !/^3\d{13}3$/.test(patch.vatNumber)) {
    throw new ApiError('الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3');
  }
  mutate(() => (db.settings = { ...db.settings, ...clone(patch), printer: { ...db.settings.printer, ...patch.printer } }));
  logActivity('settings', 'تحديث إعدادات المتجر', session.userId, new Date().toISOString(), '/settings/general');
  return clone(db.settings);
}

export async function getTaxes(): Promise<Tax[]> {
  await delay(100);
  return clone(db.taxes);
}

export async function saveTax(input: Omit<Tax, 'id'>, id?: string): Promise<Tax> {
  await delay();
  if (!input.name.trim()) throw new ApiError('اسم الضريبة مطلوب');
  if (!(input.rate >= 0 && input.rate <= 100)) throw new ApiError('النسبة يجب أن تكون بين 0 و 100');
  let tax: Tax;
  mutate(() => {
    // Only one default per type.
    if (input.isDefault) db.taxes.filter((t) => t.type === input.type).forEach((t) => (t.isDefault = false));
    if (id) {
      const found = db.taxes.find((t) => t.id === id);
      if (!found) throw new ApiError('الضريبة غير موجودة', 'NOT_FOUND');
      Object.assign(found, input);
      tax = found;
    } else {
      tax = { id: uid('tax'), ...input };
      db.taxes.push(tax);
    }
  });
  return clone(tax!);
}

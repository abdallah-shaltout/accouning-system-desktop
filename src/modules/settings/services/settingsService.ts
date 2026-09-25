import { ApiError, clone, db, delay, session, uid } from '@/mocks';
import { logActivity } from '@/mocks/backend/core';
import { mutate } from '@/mocks/persist';
import type { PaymentMethod, PaymentMethodInput, StoreSettings, Tax } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

export const getSettings = wrap('settings.getSettings', async function getSettings(): Promise<StoreSettings> {
  await delay(80);
  return clone(db.settings);
});

export const updateSettings = wrap('settings.updateSettings', async function updateSettings(patch: Partial<StoreSettings>): Promise<StoreSettings> {
  await delay();
  if (patch.storeName !== undefined && !patch.storeName.trim()) throw new ApiError('اسم المتجر مطلوب');
  if (patch.vatNumber && !/^3\d{13}3$/.test(patch.vatNumber)) {
    throw new ApiError('الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3');
  }
  mutate(() => (db.settings = { ...db.settings, ...clone(patch), printer: { ...db.settings.printer, ...patch.printer } }));
  logActivity('settings', 'تحديث إعدادات المتجر', session.userId, new Date().toISOString(), '/settings/general');
  return clone(db.settings);
});

export const getTaxes = wrap('settings.getTaxes', async function getTaxes(): Promise<Tax[]> {
  await delay(100);
  return clone(db.taxes);
});

export const saveTax = wrap('settings.saveTax', async function saveTax(input: Omit<Tax, 'id'>, id?: string): Promise<Tax> {
  await delay();
  if (!input.name.trim()) throw new ApiError('اسم الضريبة مطلوب');
  if (!(input.rate >= 0 && input.rate <= 100)) throw new ApiError('النسبة يجب أن تكون بين 0 و 100');
  if (input.category === 'E' && !input.exemptionReason?.trim()) {
    throw new ApiError('سبب الإعفاء مطلوب للضرائب المعفاة');
  }
  // `type`/`direction` must agree — OUTPUT ⇔ sales, INPUT ⇔ purchase (docs/v2/06 §3, v1/v2 field mirror).
  const direction: Tax['direction'] = input.type === 'OUTPUT' ? 'sales' : 'purchase';
  const accountRole: Tax['accountRole'] = input.type === 'OUTPUT' ? 'vatOutput' : 'vatInput';
  let tax: Tax;
  mutate(() => {
    // Only one default per type.
    if (input.isDefault) db.taxes.filter((t) => t.type === input.type).forEach((t) => (t.isDefault = false));
    const body = { ...input, direction, accountRole, exemptionReason: input.category === 'E' ? input.exemptionReason?.trim() : undefined };
    if (id) {
      const found = db.taxes.find((t) => t.id === id);
      if (!found) throw new ApiError('الضريبة غير موجودة', 'NOT_FOUND');
      Object.assign(found, body);
      tax = found;
    } else {
      tax = { id: uid('tax'), ...body };
      db.taxes.push(tax);
    }
  });
  logActivity('settings', `حفظ الضريبة "${tax!.name}"`, session.userId, new Date().toISOString(), '/settings/taxes');
  return clone(tax!);
});

export const deleteTax = wrap('settings.deleteTax', async function deleteTax(id: string): Promise<void> {
  await delay();
  const tax = db.taxes.find((t) => t.id === id);
  if (!tax) throw new ApiError('الضريبة غير موجودة', 'NOT_FOUND');
  if (tax.isDefault) throw new ApiError('لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً');
  mutate(() => (db.taxes = db.taxes.filter((t) => t.id !== id)));
});

// ---------------------------------------------------------------------------------------------
// Payment methods v2 (docs/v2/09-purchases-payments-expenses.md §2)
// ---------------------------------------------------------------------------------------------

export const getPaymentMethods = wrap('settings.getPaymentMethods', async function getPaymentMethods(): Promise<PaymentMethod[]> {
  await delay(100);
  return clone([...db.paymentMethods].sort((a, b) => a.sortOrder - b.sortOrder));
});

export const savePaymentMethod = wrap('settings.savePaymentMethod', async function savePaymentMethod(input: PaymentMethodInput, id?: string): Promise<PaymentMethod> {
  await delay();
  if (!input.name.trim()) throw new ApiError('اسم طريقة الدفع مطلوب');
  if (!(input.feePct >= 0 && input.feePct <= 100)) throw new ApiError('نسبة العمولة يجب أن تكون بين 0 و 100');
  let method: PaymentMethod;
  mutate(() => {
    if (id) {
      const found = db.paymentMethods.find((m) => m.id === id);
      if (!found) throw new ApiError('طريقة الدفع غير موجودة', 'NOT_FOUND');
      Object.assign(found, input);
      method = found;
    } else {
      method = { id: uid('pm'), canDelete: true, ...input };
      db.paymentMethods.push(method);
    }
  });
  logActivity('settings', `حفظ طريقة الدفع "${method!.name}"`, session.userId, new Date().toISOString(), '/settings/payment-methods');
  return clone(method!);
});

export const reorderPaymentMethods = wrap('settings.reorderPaymentMethods', async function reorderPaymentMethods(orderedIds: string[]): Promise<void> {
  await delay(80);
  mutate(() => orderedIds.forEach((id, i) => {
    const method = db.paymentMethods.find((m) => m.id === id);
    if (method) method.sortOrder = i + 1;
  }));
});

export const deletePaymentMethod = wrap('settings.deletePaymentMethod', async function deletePaymentMethod(id: string): Promise<void> {
  await delay();
  const method = db.paymentMethods.find((m) => m.id === id);
  if (!method) throw new ApiError('طريقة الدفع غير موجودة', 'NOT_FOUND');
  if (!method.canDelete) throw new ApiError('لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك');
  mutate(() => (db.paymentMethods = db.paymentMethods.filter((m) => m.id !== id)));
});

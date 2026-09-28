import { ApiError, clone, db, delay, session, uid } from '@/mocks';
import { logActivity, logAudit } from '@/mocks/backend/core';
import { mutate } from '@/mocks/persist';
import { countryProfile } from '@/modules/core/helpers/countryProfiles';
import type { PaymentMethod, PaymentMethodInput, StoreSettings, Tax } from '../types';

import { backendCall, usesRust } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';

export const getSettings = wrap('settings.getSettings', async function getSettings(): Promise<StoreSettings> {
  if (usesRust('settings')) return backendCall('settings_get_settings');
  await delay(80);
  return clone(db.settings);
});

export const updateSettings = wrap('settings.updateSettings', async function updateSettings(patch: Partial<StoreSettings>): Promise<StoreSettings> {
  if (usesRust('settings')) return backendCall('settings_update_settings', { patch });
  await delay();
  if (patch.storeName !== undefined && !patch.storeName.trim()) throw new ApiError('اسم المتجر مطلوب');
  if (patch.vatNumber) {
    const profile = countryProfile(patch.country ?? db.settings.country);
    if (!profile.taxId.pattern.test(patch.vatNumber)) {
      throw new ApiError(`${profile.taxId.label} يجب أن يكون ${profile.taxId.hint}`);
    }
  }
  mutate(() => (db.settings = { ...db.settings, ...clone(patch), printer: { ...db.settings.printer, ...patch.printer } }));
  logActivity('settings', 'تحديث إعدادات المتجر', session.userId, new Date().toISOString(), { name: 'settings-general' });
  return clone(db.settings);
});

export const getTaxes = wrap('settings.getTaxes', async function getTaxes(): Promise<Tax[]> {
  if (usesRust('settings')) return backendCall('settings_get_taxes');
  await delay(100);
  return clone(db.taxes);
});

export const saveTax = wrap('settings.saveTax', async function saveTax(input: Omit<Tax, 'id'>, id?: string): Promise<Tax> {
  if (usesRust('settings')) return backendCall('settings_save_tax', { input, id });
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
  logActivity('settings', `حفظ الضريبة "${tax!.name}"`, session.userId, new Date().toISOString(), { name: 'settings-taxes' });
  return clone(tax!);
});

export const deleteTax = wrap('settings.deleteTax', async function deleteTax(id: string): Promise<void> {
  if (usesRust('settings')) {
    await backendCall('settings_delete_tax', { id });
    return;
  }
  await delay();
  const tax = db.taxes.find((t) => t.id === id);
  if (!tax) throw new ApiError('الضريبة غير موجودة', 'NOT_FOUND');
  if (tax.isDefault) throw new ApiError('لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً', 'FORBIDDEN');
  const inUse = db.invoices.some((inv) => inv.lines.some((l) => l.taxId === id)) || db.purchaseOrders.some((po) => po.lines.some((l) => l.taxId === id));
  if (inUse) throw new ApiError('لا يمكن حذف ضريبة مستخدمة في مستندات مرحّلة', 'FORBIDDEN');
  mutate(() => (db.taxes = db.taxes.filter((t) => t.id !== id)));
  logAudit({ entity: 'tax', entityId: id, entityLabel: tax.name, action: 'delete', userId: session.userId, message: `حذف الضريبة "${tax.name}"`, link: { name: 'settings-taxes' }, activityKind: 'settings' });
});

// ---------------------------------------------------------------------------------------------
// Payment methods v2 (docs/v2/09-purchases-payments-expenses.md §2)
// ---------------------------------------------------------------------------------------------

export const getPaymentMethods = wrap('settings.getPaymentMethods', async function getPaymentMethods(): Promise<PaymentMethod[]> {
  if (usesRust('settings')) return backendCall('settings_get_payment_methods');
  await delay(100);
  return clone([...db.paymentMethods].sort((a, b) => a.sortOrder - b.sortOrder));
});

export const savePaymentMethod = wrap('settings.savePaymentMethod', async function savePaymentMethod(input: PaymentMethodInput, id?: string): Promise<PaymentMethod> {
  if (usesRust('settings')) return backendCall('settings_save_payment_method', { input, id });
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
  logActivity('settings', `حفظ طريقة الدفع "${method!.name}"`, session.userId, new Date().toISOString(), { name: 'settings-payment-methods' });
  return clone(method!);
});

export const reorderPaymentMethods = wrap('settings.reorderPaymentMethods', async function reorderPaymentMethods(orderedIds: string[]): Promise<void> {
  if (usesRust('settings')) {
    await backendCall('settings_reorder_payment_methods', { orderedIds });
    return;
  }
  await delay(80);
  mutate(() => orderedIds.forEach((id, i) => {
    const method = db.paymentMethods.find((m) => m.id === id);
    if (method) method.sortOrder = i + 1;
  }));
});

export const deletePaymentMethod = wrap('settings.deletePaymentMethod', async function deletePaymentMethod(id: string): Promise<void> {
  if (usesRust('settings')) {
    await backendCall('settings_delete_payment_method', { id });
    return;
  }
  await delay();
  const method = db.paymentMethods.find((m) => m.id === id);
  if (!method) throw new ApiError('طريقة الدفع غير موجودة', 'NOT_FOUND');
  if (!method.canDelete) throw new ApiError('لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك', 'FORBIDDEN');
  const inUse =
    db.invoices.some((inv) => inv.tenders?.some((t) => t.paymentMethodId === id)) ||
    db.vouchers.some((v) => 'paymentMethodId' in v && v.paymentMethodId === id) ||
    db.expenses.some((e) => e.paidFrom.kind === 'method' && e.paidFrom.paymentMethodId === id);
  if (inUse) throw new ApiError('لا يمكن حذف طريقة دفع مستخدمة في مستندات مرحّلة', 'FORBIDDEN');
  mutate(() => (db.paymentMethods = db.paymentMethods.filter((m) => m.id !== id)));
  logAudit({ entity: 'paymentMethod', entityId: id, entityLabel: method.name, action: 'delete', userId: session.userId, message: `حذف طريقة الدفع "${method.name}"`, link: { name: 'settings-payment-methods' }, activityKind: 'settings' });
});

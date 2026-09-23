import type { StoreSettings, Tax } from '@/modules/settings/types';

export const taxesFixture: Tax[] = [
  { id: 'tax-vat-out', name: 'ضريبة القيمة المضافة (مبيعات)', rate: 15, type: 'OUTPUT', isDefault: true, active: true },
  { id: 'tax-vat-in', name: 'ضريبة القيمة المضافة (مشتريات)', rate: 15, type: 'INPUT', isDefault: true, active: true },
  { id: 'tax-zero', name: 'معفى / صفري', rate: 0, type: 'OUTPUT', isDefault: false, active: true },
];

export const settingsFixture: StoreSettings = {
  storeName: 'متجر الأناقة للملابس',
  currency: 'SAR',
  vatNumber: '310123456700003',
  defaultTaxId: 'tax-vat-out',
  invoiceNumberPrefix: 'INV-',
  printer: { mode: 'thermal', thermalWidthMm: 80 },
  theme: 'light',
  address: 'الرياض - حي العليا - شارع التحلية',
  phone: '0112345678',
  commercialRegister: '1010123456',
  receiptFooter: 'شكراً لتسوقكم معنا — الاستبدال خلال 7 أيام مع الفاتورة',
};

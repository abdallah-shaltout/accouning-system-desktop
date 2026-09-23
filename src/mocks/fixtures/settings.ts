import type { PaymentMethod, StoreSettings, Tax } from '@/modules/settings/types';

/**
 * Tax v2 fixture (docs/v2/06-sales-and-pos.md §3, docs/v2/02-accounting-review.md D1/D2). The
 * Saudi preset: 15% standard VAT (S) on both sides of the books, plus a 0% zero-rated preset (Z)
 * for qualifying medicines/food-type items — matching Phase 1's pharmacy CoA add-on (4120 مبيعات
 * أدوية معفاة/صفرية). An exempt (E) preset is included too since it's a distinct VAT-return box
 * from zero-rated, per docs/v2/02-accounting-review.md D1, even though nothing in the demo catalog
 * uses it yet.
 */
export const taxesFixture: Tax[] = [
  { id: 'tax-vat-out', name: 'ضريبة القيمة المضافة 15% (مبيعات)', rate: 15, type: 'OUTPUT', isDefault: true, active: true, category: 'S', direction: 'sales', accountRole: 'vatOutput' },
  { id: 'tax-vat-in', name: 'ضريبة القيمة المضافة 15% (مشتريات)', rate: 15, type: 'INPUT', isDefault: true, active: true, category: 'S', direction: 'purchase', accountRole: 'vatInput' },
  { id: 'tax-zero-sales', name: 'صفري 0% — أدوية ومواد غذائية معينة (مبيعات)', rate: 0, type: 'OUTPUT', isDefault: false, active: true, category: 'Z', direction: 'sales', accountRole: 'vatOutput' },
  { id: 'tax-zero-purchase', name: 'صفري 0% (مشتريات)', rate: 0, type: 'INPUT', isDefault: false, active: true, category: 'Z', direction: 'purchase', accountRole: 'vatInput' },
  { id: 'tax-exempt-sales', name: 'معفى 0% (مبيعات)', rate: 0, type: 'OUTPUT', isDefault: false, active: true, category: 'E', direction: 'sales', exemptionReason: 'خدمات مالية معفاة بموجب نظام ضريبة القيمة المضافة', accountRole: 'vatOutput' },
  { id: 'tax-out-of-scope', name: 'خارج نطاق الضريبة', rate: 0, type: 'OUTPUT', isDefault: false, active: true, category: 'O', direction: 'sales' },
];

/**
 * Payment methods v2 fixture (docs/v2/09-purchases-payments-expenses.md §2 presets table). Each
 * points at its settlement account by system role — `accountFor(role)` resolves the actual account
 * id at posting time, so renumbering the CoA never breaks a tender.
 */
export const paymentMethodsFixture: PaymentMethod[] = [
  { id: 'pm-cash', name: 'نقداً', type: 'cash', accountRole: 'cash', feePct: 0, showInPos: true, showInPayments: true, sortOrder: 1, active: true, canDelete: false },
  { id: 'pm-mada', name: 'مدى', type: 'card', accountRole: 'cardClearing', feePct: 0.8, requiresReference: true, showInPos: true, showInPayments: true, sortOrder: 2, active: true, canDelete: false },
  { id: 'pm-visa', name: 'فيزا / ماستركارد', type: 'card', accountRole: 'cardClearing', feePct: 2.2, requiresReference: true, showInPos: true, showInPayments: true, sortOrder: 3, active: true, canDelete: false },
  { id: 'pm-bank-transfer', name: 'تحويل بنكي', type: 'bank_transfer', accountRole: 'bank', feePct: 0, requiresReference: true, showInPos: true, showInPayments: true, sortOrder: 4, active: true, canDelete: false },
  { id: 'pm-stc-pay', name: 'STC Pay', type: 'wallet', accountRole: 'walletClearing', feePct: 1, requiresReference: true, showInPos: true, showInPayments: true, sortOrder: 5, active: true, canDelete: false },
  { id: 'pm-credit', name: 'آجل', type: 'credit', accountRole: 'receivable', feePct: 0, showInPos: true, showInPayments: false, sortOrder: 6, active: true, canDelete: false },
];

export const settingsFixture: StoreSettings = {
  storeName: 'متجر الأناقة للملابس',
  currency: 'SAR',
  vatNumber: '310123456700003',
  defaultTaxId: 'tax-vat-out',
  invoiceNumberPrefix: 'INV-',
  printer: { mode: 'thermal', thermalWidthMm: 80 },
  theme: 'light',
  pricesIncludeTax: true,
  address: 'الرياض - حي العليا - شارع التحلية',
  phone: '0112345678',
  commercialRegister: '1010123456',
  receiptFooter: 'شكراً لتسوقكم معنا — الاستبدال خلال 7 أيام مع الفاتورة',
};

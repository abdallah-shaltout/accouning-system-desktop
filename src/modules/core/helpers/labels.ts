/** Arabic labels + badge tones for enums used across modules. */
import type { InvoiceStatus, PaymentStatus, SalePaymentMethod } from '@/modules/invoices/types';
import type { PaymentMethod } from '@/modules/payments/types';
import type { StockAdjustmentType, StockMovementReason } from '@/modules/products/types';
import type { PurchaseStatus } from '@/modules/purchases/types';
import type { Role } from '@/modules/users/types';

export type Tone = 'neutral' | 'success' | 'warning' | 'danger' | 'primary';

export const ROLE_LABEL: Record<Role, string> = {
  admin: 'مدير النظام',
  manager: 'مدير المتجر',
  accountant: 'محاسب',
  cashier: 'كاشير',
  storekeeper: 'أمين مخزن',
};

export const INVOICE_STATUS: Record<InvoiceStatus, { label: string; tone: Tone }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  COMPLETED: { label: 'مكتملة', tone: 'success' },
  REFUNDED: { label: 'مسترجعة', tone: 'danger' },
};

export const PAYMENT_STATUS: Record<PaymentStatus, { label: string; tone: Tone }> = {
  UNPAID: { label: 'غير مدفوعة', tone: 'neutral' },
  PARTIALLY_PAID: { label: 'مدفوعة جزئياً', tone: 'warning' },
  PAID: { label: 'مدفوعة', tone: 'success' },
};

export const PURCHASE_STATUS: Record<PurchaseStatus, { label: string; tone: Tone }> = {
  DRAFT: { label: 'مسودة', tone: 'neutral' },
  ORDERED: { label: 'مرسل للمورد', tone: 'warning' },
  RECEIVED: { label: 'مستلم', tone: 'success' },
  CANCELED: { label: 'ملغي', tone: 'danger' },
};

export const SALE_METHOD_LABEL: Record<SalePaymentMethod, string> = {
  cash: 'نقداً',
  card: 'بطاقة (مدى/فيزا)',
  bank_transfer: 'تحويل بنكي',
  credit: 'آجل',
};

export const PAYMENT_METHOD_LABEL: Record<PaymentMethod, string> = {
  cash: 'نقداً',
  card: 'بطاقة',
  bank_transfer: 'تحويل بنكي',
};

export const PHONE_LABEL: Record<'mobile' | 'work' | 'whatsapp', string> = {
  mobile: 'جوال',
  work: 'عمل',
  whatsapp: 'واتساب',
};

export const ADJUSTMENT_TYPE: Record<StockAdjustmentType, { label: string; tone: Tone }> = {
  STOCK_IN: { label: 'إدخال مخزون', tone: 'success' },
  LOSS: { label: 'إتلاف / فقد', tone: 'danger' },
  STOCKTAKE: { label: 'جرد', tone: 'primary' },
};

export const MOVEMENT_REASON: Record<StockMovementReason, string> = {
  sale: 'بيع',
  purchase: 'شراء',
  stock_in: 'إدخال مخزون',
  loss: 'إتلاف / فقد',
  stocktake: 'تسوية جرد',
  refund: 'مرتجع مبيعات',
  purchase_return: 'مرتجع مشتريات',
  transfer_out: 'تحويل صادر لفرع آخر',
  transfer_in: 'تحويل وارد من فرع آخر',
};

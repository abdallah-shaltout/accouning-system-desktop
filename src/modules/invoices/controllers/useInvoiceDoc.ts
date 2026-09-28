import { computed, type Ref } from 'vue';
import { formatAddress, formatDate, formatDateLong, formatDateTime, formatDigits, formatTime } from '@/modules/core/helpers/format';
import { PAYMENT_STATUS, SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { countryProfile } from '@/modules/core/helpers/countryProfiles';
import { tafqit } from '@/modules/core/helpers/tafqit';
import { changeDue, invoiceOutstanding, round2 } from '../helpers/totals';
import { zatcaQrBase64 } from '../helpers/zatcaQr';
import type { PrintData } from '../services/invoiceService';

export interface DocTotalRow {
  key: 'sub' | 'discount' | 'tax' | 'grand' | 'refund';
  label: string;
  /** Rendered after the label as "(N%)" when present. */
  rate?: number;
  value: number;
  /** Shown with a leading minus. */
  negative?: boolean;
}

export interface DocLine {
  id: string;
  /** 1-based position. */
  no: number;
  name: string;
  qty: number;
  price: number;
  discount: number;
  /** qty × price − line discount (what the thermal receipt prints per line). */
  amount: number;
  /** This line's VAT: the posted snapshot when present, else the legacy A4 formula. */
  vat: number;
  /** Line total including VAT, after the invoice discount share. */
  total: number;
}

/**
 * Plan 22: the one place an invoice template gets its printable values from. Templates only lay
 * these out — none of them re-implements money math (UI rule 9). Mirrors what `InvoiceA4.vue` and
 * `InvoiceThermal.vue` compute, so every template prints the same numbers.
 */
export function useInvoiceDoc(data: Ref<PrintData>) {
  const inv = computed(() => data.value.invoice);
  const store = computed(() => data.value.settings);
  const customer = computed(() => data.value.customer);
  const profile = computed(() => countryProfile(store.value.country));
  const isB2B = computed(() => !!customer.value?.vatNumber);
  const isZatca = computed(() => profile.value.eInvoice === 'zatca-phase1');

  const title = computed(() => (isB2B.value ? profile.value.invoiceTitles.b2b : profile.value.invoiceTitles.b2c));
  const titleEn = computed(() => (isZatca.value ? (isB2B.value ? 'TAX INVOICE' : 'SIMPLIFIED TAX INVOICE') : ''));

  // v2 doc 18.D: ZATCA QR only for a Saudi ('zatca-phase1') company — never rendered for Egypt.
  const qr = computed(() =>
    isZatca.value
      ? zatcaQrBase64({
          sellerName: store.value.storeName,
          vatNumber: store.value.vatNumber ?? '',
          timestamp: inv.value.date,
          invoiceTotal: inv.value.grandTotal,
          vatTotal: inv.value.taxAmount,
        })
      : null,
  );

  const lines = computed<DocLine[]>(() => {
    const discFactor = 1 - inv.value.discountRate / 100;
    const rate = inv.value.taxRate / 100;
    return inv.value.lines.map((l, i) => {
      const amount = round2(l.qty * l.price - l.discount);
      const posted = l.net !== undefined && l.vat !== undefined;
      const vat = posted ? (l.vat as number) : round2(amount * discFactor * rate);
      const total = posted ? round2((l.net as number) + (l.vat as number)) : round2(amount * discFactor * (1 + rate));
      return { id: l.id, no: i + 1, name: l.name, qty: l.qty, price: l.price, discount: l.discount, amount, vat, total };
    });
  });

  const itemCount = computed(() => inv.value.lines.reduce((a, l) => a + l.qty, 0));
  const taxable = computed(() => round2(inv.value.subTotal - inv.value.discountAmount));
  const outstanding = computed(() => invoiceOutstanding(inv.value));
  const change = computed(() => (inv.value.tenderedAmount ? changeDue(inv.value.grandTotal, inv.value.tenderedAmount) : 0));

  /** The totals block, in print order — every template renders these rows, only styled differently. */
  const totals = computed<DocTotalRow[]>(() => {
    const i = inv.value;
    const rows: DocTotalRow[] = [{ key: 'sub', label: 'المجموع', value: i.subTotal }];
    if (i.discountAmount > 0) rows.push({ key: 'discount', label: 'الخصم', rate: i.discountRate || undefined, value: i.discountAmount, negative: true });
    rows.push({ key: 'tax', label: 'ضريبة القيمة المضافة', rate: i.taxRate, value: i.taxAmount });
    rows.push({ key: 'grand', label: 'الإجمالي', value: i.grandTotal });
    if (i.refundedAmount > 0) rows.push({ key: 'refund', label: 'المرتجع', value: i.refundedAmount, negative: true });
    return rows;
  });
  /** Totals without the grand-total row (templates that show the grand total as a hero figure). */
  const breakdown = computed(() => totals.value.filter((r) => r.key !== 'grand'));

  const storeAddress = computed(() => formatAddress(store.value.nationalAddress) || store.value.address || '');
  const customerAddress = computed(() => formatAddress(customer.value?.structuredAddress) || customer.value?.address || '');
  /** First letters of the store's first two words — the monogram some templates draw instead of a logo. */
  const monogram = computed(() =>
    store.value.storeName
      .trim()
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => w.replace(/^ال/, '').charAt(0))
      .join(''),
  );

  return {
    inv,
    store,
    customer,
    profile,
    isB2B,
    isZatca,
    title,
    titleEn,
    qr,
    lines,
    totals,
    breakdown,
    itemCount,
    taxable,
    outstanding,
    change,
    storeAddress,
    customerAddress,
    monogram,
    currency: computed(() => inv.value.currency),
    number: computed(() => formatDigits(inv.value.number)),
    date: computed(() => formatDate(inv.value.date)),
    dateLong: computed(() => formatDateLong(inv.value.date)),
    dateTime: computed(() => formatDateTime(inv.value.date)),
    time: computed(() => formatTime(inv.value.date)),
    dueDate: computed(() => (inv.value.dueDate ? formatDate(inv.value.dueDate) : '')),
    paymentLabel: computed(() => SALE_METHOD_LABEL[inv.value.paymentMethod]),
    paymentStatus: computed(() => PAYMENT_STATUS[inv.value.paymentStatus]),
    isRefunded: computed(() => inv.value.status === 'REFUNDED' || inv.value.refundedAmount > 0),
    amountInWords: computed(() => tafqit(inv.value.grandTotal, { currency: inv.value.currency ?? store.value.currency })),
    cashier: computed(() => data.value.cashierName),
    footer: computed(() => store.value.receiptFooter ?? ''),
    note: computed(() => inv.value.note ?? ''),
    sample: computed(() => !!data.value.sample),
  };
}

export type InvoiceDoc = ReturnType<typeof useInvoiceDoc>;

import type { SalePaymentMethod } from '@/modules/invoices/types';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import { db } from '../db';
import { supplierByCategory } from '../fixtures/people';
import { logActivity, postJournal } from '../backend/core';
import { recordStockAdjustment } from '../backend/inventory';
import { recordPayment } from '../backend/payments';
import { purchaseOutstanding, recordPurchaseReturn, savePurchase, cancelPurchase } from '../backend/purchases';
import { recordRefund, recordSale } from '../backend/sales';
import { createRandom, round2, type RandomSource } from '../utils';

const HISTORY_DAYS = 75;
const ADMIN = 'usr-1';
const MANAGER = 'usr-2';
const ACCOUNTANT = 'usr-3';
const REFUND_REASONS = ['المقاس غير مناسب', 'عيب في الخياطة', 'تغيير رأي العميل', 'اللون مختلف عن المتوقع'];

/**
 * Builds a realistic shop history by *replaying operations* through the same posting rules the
 * services use — so stock levels, journal entries, balances and every report agree with each other.
 * Deterministic (seeded PRNG) apart from "today", which depends on the current clock.
 *
 * Needs accounts + catalog + people + settings seeded first (their opening balances/stock too).
 */
export function seedHistory(now = new Date()): void {
  const rnd = createRandom(20260923);

  const day0 = new Date(now);
  day0.setDate(day0.getDate() - HISTORY_DAYS);
  day0.setHours(0, 0, 0, 0);
  const at = (day: Date, hour: number, minute = 0) => {
    const d = new Date(day);
    d.setHours(hour, minute, rnd.int(0, 59), 0);
    return d.toISOString();
  };

  const cashiers = ['usr-4', 'usr-4', 'usr-5', 'usr-2'];
  const activeCustomers = db.customers.filter((c) => c.active);
  const stocked = () => db.products.filter((p) => p.active && (p.type === 'service' || p.stockQty > 0));

  // --- Daily operations -------------------------------------------------------------------------
  for (let d = 1; d <= HISTORY_DAYS; d++) {
    const day = new Date(day0);
    day.setDate(day0.getDate() + d);
    const isToday = d === HISTORY_DAYS;
    const dom = day.getDate();
    const weekend = day.getDay() === 4 || day.getDay() === 5; // Thu / Fri are the busy days
    const events: { time: string; run: () => void }[] = [];

    // Monthly fixed expenses.
    if (dom === 1) {
      const t = at(day, 9, 5);
      events.push({ time: t, run: () => expense(t, 'إيجار المحل — شهر ' + (day.getMonth() + 1), '5300', 9000, '1120', ACCOUNTANT) });
    }
    if (dom === 27) {
      const t = at(day, 9, 10);
      events.push({ time: t, run: () => expense(t, 'رواتب الموظفين — شهر ' + (day.getMonth() + 1), '5600', 18500, '1120', ACCOUNTANT) });
    }
    if (dom === 10) {
      const t = at(day, 11, 0);
      const amount = rnd.int(1100, 1900);
      events.push({ time: t, run: () => expense(t, 'فاتورة الكهرباء والمياه', '5300', amount, '1110', ACCOUNTANT) });
    }
    if (dom === 15) {
      const t = at(day, 12, 0);
      const amount = rnd.int(250, 700);
      events.push({ time: t, run: () => expense(t, 'قرطاسية ومستلزمات مكتبية', '5500', amount, '1110', ACCOUNTANT) });
    }
    if (d === 40) {
      const t = at(day, 20, 0);
      events.push({ time: t, run: () => expense(t, 'مسحوبات شخصية للمالك', '3400', 5000, '1110', ADMIN) });
    }

    // Restock every 8 days: one PO per supplier for anything running low.
    if (d % 8 === 3 && !isToday) {
      const t = at(day, 9, 30);
      events.push({ time: t, run: () => restock(t, rnd) });
    }

    // Sales.
    const saleCount = weekend ? rnd.int(9, 14) : rnd.int(5, 10);
    const open = new Date(day).setHours(10, 0, 0, 0);
    let close = new Date(day).setHours(22, 30, 0, 0);
    if (isToday) close = Math.min(close, now.getTime() - 2 * 60_000);
    const windowStart = close - open < 30 * 60_000 ? now.getTime() - 45 * 60_000 : open;
    const todayCount = isToday ? Math.max(3, Math.round((saleCount * (close - windowStart)) / (12.5 * 3600_000))) : saleCount;
    for (let i = 0; i < todayCount; i++) {
      const time = new Date(windowStart + rnd.next() * Math.max(close - windowStart, 60_000)).toISOString();
      events.push({ time, run: () => sale(time, rnd, cashiers, activeCustomers, stocked) });
    }

    // Collections from customers every 5 days.
    if (d % 5 === 0) {
      const t = at(day, 13, 15);
      events.push({ time: t, run: () => collect(t, day, rnd) });
    }
    // Supplier payments every 6 days.
    if (d % 6 === 2) {
      const t = at(day, 10, 30);
      events.push({ time: t, run: () => paySuppliers(t, day, rnd) });
    }
    // Occasional returns.
    if (rnd.chance(0.3) && !isToday) {
      const t = at(day, rnd.int(12, 21), rnd.int(0, 59));
      events.push({ time: t, run: () => refund(t, rnd) });
    }
    // Weekly cash deposit to the bank.
    if (day.getDay() === 6 && !isToday) {
      const t = at(day, 23, 0);
      events.push({ time: t, run: () => depositCash(t) });
    }
    // Stocktakes and write-offs.
    if (d === 30 || d === 60) {
      const t = at(day, 9, 15);
      const categoryId = d === 30 ? 'cat-men' : 'cat-acc';
      events.push({ time: t, run: () => stocktake(t, categoryId, rnd) });
    }
    if (d % 21 === 7) {
      const t = at(day, 21, 45);
      events.push({ time: t, run: () => writeOff(t, rnd) });
    }

    events.sort((a, b) => a.time.localeCompare(b.time));
    for (const e of events) e.run();
  }

  // --- Leave a few open items for the UI --------------------------------------------------------
  const today = new Date(now);
  const lastConfirmed = [...db.purchaseOrders].reverse().find((p) => p.status === 'CONFIRMED');
  if (lastConfirmed) {
    const line = lastConfirmed.lines[0];
    const product = db.products.find((p) => p.id === line.productId);
    if (product && product.stockQty >= 2) {
      recordPurchaseReturn(
        { purchaseOrderId: lastConfirmed.id, reason: 'عيوب تصنيع في الدفعة', lines: [{ productId: line.productId, qty: Math.min(2, line.qty) }] },
        MANAGER,
        new Date(today.getTime() - 60_000).toISOString(), // after today's last sale, so stock history stays chronological
      );
    }
  }
  const low = db.products.filter((p) => p.type === 'product' && p.stockQty <= (p.minStock ?? 0) * 1.5).slice(0, 4);
  const draftSupplier = low.length ? supplierByCategory[low[0].categoryId!] : 'sup-1';
  savePurchase(
    {
      supplierId: draftSupplier,
      date: new Date(today.getTime() - 3 * 3600_000).toISOString(),
      note: 'طلبية مقترحة للأصناف منخفضة المخزون',
      confirm: false,
      lines: (low.length ? low.filter((p) => supplierByCategory[p.categoryId!] === draftSupplier) : db.products.slice(0, 2)).map((p) => ({
        productId: p.id,
        qty: (p.minStock ?? 5) * 3,
        costPrice: p.costPrice,
      })),
    },
    MANAGER,
  );
  const canceled = savePurchase(
    {
      supplierId: 'sup-4',
      date: new Date(today.getTime() - 9 * 86400_000).toISOString(),
      note: 'تم الإلغاء — تأخر المورد',
      confirm: false,
      lines: [{ productId: 'prd-28', qty: 5, costPrice: 175 }],
    },
    MANAGER,
  );
  cancelPurchase(canceled.id, MANAGER);
  recordStockAdjustment(
    {
      type: 'STOCKTAKE',
      date: new Date(today.getTime() - 2 * 3600_000).toISOString(),
      note: 'جرد قسم الأحذية — بانتظار الاعتماد',
      lines: db.products
        .filter((p) => p.categoryId === 'cat-shoes')
        .map((p) => ({ productId: p.id, countedQty: Math.max(0, p.stockQty - (rnd.chance(0.4) ? 1 : 0)) })),
    },
    MANAGER,
    true,
  );

  // Events were appended roughly in time order; make it exact (services read it newest-first).
  db.activity.sort((a, b) => a.date.localeCompare(b.date));
}

// --- helpers -------------------------------------------------------------------------------------

function expense(date: string, description: string, debitCode: string, amount: number, creditCode: string, userId: string) {
  const entry = postJournal({
    date,
    description,
    type: 'MANUAL',
    lines: [
      { code: debitCode, debit: amount },
      { code: creditCode, credit: amount },
    ],
    createdBy: userId,
  });
  logActivity('journal', `قيد ${entry.number} — ${description}`, userId, entry.date, `/accounting/journal/${entry.id}`);
}

function sale(
  time: string,
  rnd: RandomSource,
  cashiers: string[],
  activeCustomers: ReturnType<typeof db.customers.filter>,
  stocked: () => typeof db.products,
) {
  const cashierId = rnd.pick(cashiers);
  const cashier = db.users.find((u) => u.id === cashierId)!;
  const available = stocked().filter((p) => p.type === 'product');
  if (!available.length) return;

  const lineCount = rnd.chance(0.55) ? 1 : rnd.int(2, 4);
  const chosen = new Set<string>();
  const lines: { productId: string; qty: number; price: number }[] = [];
  for (let i = 0; i < lineCount; i++) {
    const product = rnd.pick(available);
    if (chosen.has(product.id)) continue;
    chosen.add(product.id);
    const qty = Math.min(product.stockQty, rnd.chance(0.75) ? 1 : rnd.int(2, 3));
    const listPrice = cashier.priceListId ? product.prices?.find((x) => x.priceListId === cashier.priceListId)?.value : undefined;
    lines.push({ productId: product.id, qty, price: listPrice ?? product.price });
  }
  if (rnd.chance(0.08)) {
    const service = rnd.pick(db.products.filter((p) => p.type === 'service'));
    lines.push({ productId: service.id, qty: 1, price: service.price });
  }

  const customer = rnd.chance(0.35) ? rnd.pick(activeCustomers) : undefined;
  let paymentMethod: SalePaymentMethod;
  let partial = false;
  if (!customer) paymentMethod = rnd.chance(0.55) ? 'cash' : 'card';
  else {
    const r = rnd.next();
    const creditFriendly = customer.type === 'company';
    if (r < (creditFriendly ? 0.55 : 0.2)) paymentMethod = 'credit';
    else if (r < (creditFriendly ? 0.7 : 0.35)) {
      paymentMethod = 'cash';
      partial = true;
    } else paymentMethod = rnd.chance(0.5) ? 'cash' : 'card';
  }
  const discountRate = rnd.chance(0.15) ? Math.min(cashier.maxDiscount, rnd.pick([5, 10])) : 0;

  // Rough grand total to size cash tendered / partial payments (backend recomputes exactly).
  const approx = round2(lines.reduce((a, l) => a + l.qty * l.price, 0) * (1 - discountRate / 100) * 1.15);
  const paidAmount = partial ? Math.floor((approx * rnd.int(30, 70)) / 100 / 10) * 10 : approx + 1;
  const tenderedAmount = paymentMethod === 'cash' && !partial ? Math.ceil(approx / 50) * 50 : undefined;

  recordSale({ customerId: customer?.id, lines, discountRate, paymentMethod, paidAmount, tenderedAmount }, cashierId, time);
}

function collect(time: string, day: Date, rnd: RandomSource) {
  const cutoff = new Date(day.getTime() - 3 * 86400_000).toISOString();
  const open = db.invoices.filter((i) => i.customerId && i.status === 'COMPLETED' && invoiceOutstanding(i) > 0 && i.date < cutoff);
  for (const inv of open) {
    if (!rnd.chance(0.6)) continue;
    const outstanding = invoiceOutstanding(inv);
    const amount = rnd.chance(0.7) ? outstanding : round2(Math.max(10, Math.floor((outstanding * rnd.int(40, 80)) / 100)));
    recordPayment(
      { date: time, type: 'RECEIVED', targetType: 'customer', targetId: inv.customerId!, targetRef: inv.id, amount: Math.min(amount, outstanding), method: rnd.chance(0.5) ? 'cash' : 'bank_transfer' },
      ACCOUNTANT,
    );
  }
}

function paySuppliers(time: string, day: Date, rnd: RandomSource) {
  const cutoff = new Date(day.getTime() - 4 * 86400_000).toISOString();
  for (const po of db.purchaseOrders.filter((p) => p.status === 'CONFIRMED' && purchaseOutstanding(p) > 0 && p.date < cutoff)) {
    if (!rnd.chance(0.65)) continue;
    const outstanding = purchaseOutstanding(po);
    const amount = rnd.chance(0.6) ? outstanding : round2(Math.floor((outstanding * 0.5) / 100) * 100 || outstanding);
    recordPayment(
      { date: time, type: 'PAID', targetType: 'supplier', targetId: po.supplierId, targetRef: po.id, amount: Math.min(amount, outstanding), method: 'bank_transfer', note: 'دفعة من الحساب' },
      ACCOUNTANT,
    );
  }
}

function restock(time: string, rnd: RandomSource) {
  const bySupplier = new Map<string, { productId: string; qty: number; costPrice: number }[]>();
  for (const p of db.products.filter((x) => x.type === 'product' && x.active)) {
    const min = p.minStock ?? 5;
    if (p.stockQty > min * 2) continue;
    const supplierId = supplierByCategory[p.categoryId!];
    const qty = Math.max(min * 4 - p.stockQty, min * 2);
    const costPrice = round2(p.costPrice * (0.96 + rnd.next() * 0.08));
    bySupplier.set(supplierId, [...(bySupplier.get(supplierId) ?? []), { productId: p.id, qty: Math.round(qty), costPrice }]);
  }
  for (const [supplierId, lines] of bySupplier) {
    savePurchase({ supplierId, date: time, lines, confirm: true }, MANAGER);
  }
}

function refund(time: string, rnd: RandomSource) {
  const since = new Date(new Date(time).getTime() - 5 * 86400_000).toISOString();
  const candidates = db.invoices.filter((i) => i.status === 'COMPLETED' && i.date >= since && i.date < time && i.refundedAmount === 0);
  if (!candidates.length) return;
  const inv = rnd.pick(candidates);
  const line = rnd.pick(inv.lines);
  recordRefund({ invoiceId: inv.id, reason: rnd.pick(REFUND_REASONS), lines: [{ invoiceLineId: line.id, qty: 1 }] }, MANAGER, time);
}

function depositCash(time: string) {
  const cash = db.journalEntries
    .flatMap((e) => e.lines)
    .filter((l) => l.accountId === 'acc-1110')
    .reduce((a, l) => a + l.debit - l.credit, 0);
  const amount = Math.floor((cash - 5000) / 1000) * 1000;
  if (amount <= 0) return;
  expense(time, 'إيداع النقدية في البنك', '1120', amount, '1110', ACCOUNTANT);
}

function stocktake(time: string, categoryId: string, rnd: RandomSource) {
  recordStockAdjustment(
    {
      type: 'STOCKTAKE',
      date: time,
      note: `جرد دوري — ${db.categories.find((c) => c.id === categoryId)?.name}`,
      lines: db.products
        .filter((p) => p.categoryId === categoryId && p.type === 'product')
        .map((p) => ({ productId: p.id, countedQty: Math.max(0, p.stockQty + (rnd.chance(0.3) ? rnd.int(-2, 1) : 0)) })),
    },
    MANAGER,
  );
}

function writeOff(time: string, rnd: RandomSource) {
  const candidates = db.products.filter((p) => p.type === 'product' && p.stockQty > 3);
  const product = rnd.pick(candidates);
  recordStockAdjustment(
    { type: 'LOSS', date: time, note: rnd.pick(['تلف أثناء العرض', 'بقع لا يمكن إزالتها', 'فقد أثناء الجرد']), lines: [{ productId: product.id, qtyChange: 1 }] },
    MANAGER,
  );
}

export { ADMIN, MANAGER, ACCOUNTANT };

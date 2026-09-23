import { clone, db, delay, localDateKey, round2, sum } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import type { Invoice } from '@/modules/invoices/types';
import type { Product } from '@/modules/products/types';
import type { SystemRole } from '@/modules/accounting/types';
import type { ActivityEntry, DashboardSummary } from '../types';

function accountBalance(role: SystemRole): number {
  const id = accountFor(role).id;
  let total = 0;
  for (const entry of db.journalEntries) for (const line of entry.lines) if (line.accountId === id) total += line.debit - line.credit;
  return round2(total);
}

/** KPI numbers for the home screen, computed from the same mock data every other screen uses. */
export async function getDashboardSummary(): Promise<DashboardSummary> {
  await delay();
  const today = localDateKey(new Date());
  const sold = db.invoices.filter((i) => i.status !== 'DRAFT');
  const todays = sold.filter((i) => localDateKey(i.date) === today);
  const todayRefunds = db.refunds.filter((r) => localDateKey(r.date) === today);
  const unpaid = sold.filter((i) => invoiceOutstanding(i) > 0);
  const cashOnHand = accountBalance('cash');
  const bankBalance = accountBalance('bank');

  const trend: DashboardSummary['salesTrend'] = [];
  for (let d = 13; d >= 0; d--) {
    const day = new Date();
    day.setDate(day.getDate() - d);
    const key = localDateKey(day);
    const gross = sum(sold.filter((i) => localDateKey(i.date) === key), (i) => i.grandTotal);
    const refunds = sum(db.refunds.filter((r) => localDateKey(r.date) === key), (r) => r.grandTotal);
    trend.push({ date: key, total: round2(gross - refunds) });
  }

  return {
    todaySales: round2(sum(todays, (i) => i.grandTotal) - sum(todayRefunds, (r) => r.grandTotal)),
    todayInvoiceCount: todays.length,
    unpaidInvoiceCount: unpaid.length,
    unpaidInvoiceTotal: sum(unpaid, invoiceOutstanding),
    lowStockCount: lowStock().length,
    cashPosition: round2(cashOnHand + bankBalance),
    cashOnHand,
    bankBalance,
    salesTrend: trend,
  };
}

function lowStock(): Product[] {
  return db.products.filter((p) => p.active && p.type === 'product' && p.stockQty <= (p.minStock ?? 0));
}

export async function getLowStockProducts(limit = 6): Promise<Product[]> {
  await delay();
  return clone(lowStock().sort((a, b) => a.stockQty / (a.minStock || 1) - b.stockQty / (b.minStock || 1)).slice(0, limit));
}

export async function getRecentInvoices(limit = 8): Promise<(Invoice & { customerName?: string })[]> {
  await delay();
  return [...db.invoices]
    .sort((a, b) => b.date.localeCompare(a.date))
    .slice(0, limit)
    .map((i) => ({ ...clone(i), customerName: db.customers.find((c) => c.id === i.customerId)?.name }));
}

export async function getRecentActivity(limit = 12): Promise<(ActivityEntry & { userName?: string })[]> {
  await delay();
  return db.activity
    .filter((a) => a.kind !== 'auth')
    .sort((a, b) => b.date.localeCompare(a.date))
    .slice(0, limit)
    .map((a) => ({ ...clone(a), userName: db.users.find((u) => u.id === a.userId)?.name }));
}

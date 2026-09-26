import { clone, db, delay, localDateKey, round2, sum } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { on } from '@/mocks/events';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import type { Invoice } from '@/modules/invoices/types';
import type { Product } from '@/modules/products/types';
import type { SystemRole } from '@/modules/accounting/types';
import type { StockTransfer } from '@/modules/products/types';
import type { ApprovalRequest } from '@/modules/approvals/types';
import type { ActivityEntry, DashboardSummary } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

function accountBalance(role: SystemRole): number {
  const id = accountFor(role).id;
  let total = 0;
  for (const entry of db.journalEntries) for (const line of entry.lines) if (line.accountId === id) total += line.debit - line.credit;
  return round2(total);
}

/** KPI numbers for the home screen, computed from the same mock data every other screen uses. */
export const getDashboardSummary = wrap('core.getDashboardSummary', async function getDashboardSummary(): Promise<DashboardSummary> {
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
});

function lowStock(): Product[] {
  return db.products.filter((p) => p.active && p.type === 'product' && p.stockMode !== 'none' && p.stockQty <= (p.minStock ?? 0));
}

export const getLowStockProducts = wrap('core.getLowStockProducts', async function getLowStockProducts(limit = 6): Promise<Product[]> {
  await delay();
  return clone(lowStock().sort((a, b) => a.stockQty / (a.minStock || 1) - b.stockQty / (b.minStock || 1)).slice(0, limit));
});

export const getRecentInvoices = wrap('core.getRecentInvoices', async function getRecentInvoices(limit = 8): Promise<(Invoice & { customerName?: string })[]> {
  await delay();
  return [...db.invoices]
    .sort((a, b) => b.date.localeCompare(a.date))
    .slice(0, limit)
    .map((i) => ({ ...clone(i), customerName: db.customers.find((c) => c.id === i.customerId)?.name }));
});

export const getRecentActivity = wrap('core.getRecentActivity', async function getRecentActivity(limit = 12): Promise<(ActivityEntry & { userName?: string })[]> {
  await delay();
  return db.activity
    .filter((a) => a.kind !== 'auth')
    .sort((a, b) => b.date.localeCompare(a.date))
    .slice(0, limit)
    .map((a) => ({ ...clone(a), userName: db.users.find((u) => u.id === a.userId)?.name }));
});

// =================================================================================================
// v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part B "simpler home") — 4 KPIs with
// period-over-period comparison + sparkline, one comparison chart, top products/customers.
// =================================================================================================

export type HomePeriod = 'today' | 'week' | 'month';

function periodRange(period: HomePeriod, today = new Date()): { from: string; to: string; prevFrom: string; prevTo: string } {
  const end = new Date(today);
  const start = new Date(today);
  if (period === 'today') {
    // start === end (today)
  } else if (period === 'week') {
    start.setDate(start.getDate() - 6);
  } else {
    start.setDate(start.getDate() - 29);
  }
  const spanDays = Math.round((end.getTime() - start.getTime()) / 86_400_000) + 1;
  const prevEnd = new Date(start);
  prevEnd.setDate(prevEnd.getDate() - 1);
  const prevStart = new Date(prevEnd);
  prevStart.setDate(prevStart.getDate() - spanDays + 1);
  return { from: localDateKey(start), to: localDateKey(end), prevFrom: localDateKey(prevStart), prevTo: localDateKey(prevEnd) };
}

function invoicesInRange(from: string, to: string): Invoice[] {
  return db.invoices.filter((i) => i.status !== 'DRAFT' && localDateKey(i.date) >= from && localDateKey(i.date) <= to);
}

function netSalesFor(from: string, to: string): number {
  const gross = sum(invoicesInRange(from, to), (i) => i.grandTotal);
  const refunds = sum(db.refunds.filter((r) => localDateKey(r.date) >= from && localDateKey(r.date) <= to), (r) => r.grandTotal);
  return round2(gross - refunds);
}

/** Gross profit = Σ(line.net − qty × costPrice) over invoices in range, minus the same for refunds. */
function grossProfitFor(from: string, to: string): { profit: number; revenue: number } {
  let profit = 0;
  let revenue = 0;
  for (const inv of invoicesInRange(from, to)) {
    for (const line of inv.lines) {
      const net = line.net ?? round2(line.qty * line.price - line.discount);
      revenue += net;
      profit += net - line.qty * line.costPrice;
    }
  }
  for (const r of db.refunds.filter((r) => localDateKey(r.date) >= from && localDateKey(r.date) <= to)) {
    const original = db.invoices.find((i) => i.id === r.invoiceId);
    for (const rl of r.lines) {
      const origLine = original?.lines.find((l) => l.id === rl.invoiceLineId);
      if (!origLine) continue;
      const unitNet = (origLine.net ?? round2(origLine.qty * origLine.price - origLine.discount)) / (origLine.qty || 1);
      const net = round2(unitNet * rl.qty);
      revenue -= net;
      profit -= net - rl.qty * origLine.costPrice;
    }
  }
  return { profit: round2(profit), revenue: round2(revenue) };
}

function receivablesOverdue(): number {
  return round2(
    sum(
      db.invoices.filter((i) => i.status !== 'DRAFT' && i.status !== 'REFUNDED' && i.dueDate && invoiceOutstanding(i) > 0 && i.dueDate < new Date().toISOString()),
      invoiceOutstanding,
    ),
  );
}

export interface HomeKpi {
  value: number;
  previous: number;
  changePct: number | null;
  sparkline: number[];
}

export interface HomeKpis {
  netSales: HomeKpi;
  grossProfit: HomeKpi & { marginPct: number };
  cash: HomeKpi;
  receivables: HomeKpi & { overdue: number };
  /** Current period's daily net sales, oldest first — the home's one comparison chart. */
  salesTrend: { date: string; total: number; previousTotal?: number }[];
}

function changePct(current: number, previous: number): number | null {
  if (previous === 0) return current === 0 ? 0 : null;
  return round2(((current - previous) / Math.abs(previous)) * 100);
}

/** Daily net sales sparkline for the last `days` days ending on `to` (used for both the KPI mini-chart and the trend chart). */
function dailySeries(days: number, to: Date): number[] {
  const out: number[] = [];
  for (let d = days - 1; d >= 0; d--) {
    const day = new Date(to);
    day.setDate(day.getDate() - d);
    const key = localDateKey(day);
    out.push(netSalesFor(key, key));
  }
  return out;
}

/** Home KPIs (docs/v2/11 Part B.3): 4 KPIs with period-over-period comparison + sparkline. */
export const getHomeKpis = wrap('core.getHomeKpis', async function getHomeKpis(period: HomePeriod = 'today'): Promise<HomeKpis> {
  await delay();
  const { from, to, prevFrom, prevTo } = periodRange(period);
  const netSales = netSalesFor(from, to);
  const prevNetSales = netSalesFor(prevFrom, prevTo);
  const gp = grossProfitFor(from, to);
  const prevGp = grossProfitFor(prevFrom, prevTo);
  const cashOnHand = accountBalance('cash');
  const bankBalance = accountBalance('bank');
  const cash = round2(cashOnHand + bankBalance);
  // Cash has no meaningful "previous period" snapshot in this mock (no historical balance ledger
  // query beyond `to` a date) — approximate with the balance as of the previous period's end.
  const cashPrev = round2(accountBalanceAsOf('cash', prevTo) + accountBalanceAsOf('bank', prevTo));
  const receivablesTotal = round2(sum(db.invoices.filter((i) => i.status !== 'DRAFT' && i.status !== 'REFUNDED'), invoiceOutstanding));
  const receivablesPrev = round2(sum(db.invoices.filter((i) => i.status !== 'DRAFT' && i.status !== 'REFUNDED' && localDateKey(i.date) <= prevTo), invoiceOutstanding));

  const spark = dailySeries(14, new Date());

  const trend: HomeKpis['salesTrend'] = [];
  const spanDays = Math.round((new Date(to).getTime() - new Date(from).getTime()) / 86_400_000) + 1;
  for (let d = spanDays - 1; d >= 0; d--) {
    const day = new Date(to);
    day.setDate(day.getDate() - d);
    const key = localDateKey(day);
    const prevDay = new Date(prevTo);
    prevDay.setDate(prevDay.getDate() - d);
    const prevKey = localDateKey(prevDay);
    trend.push({ date: key, total: netSalesFor(key, key), previousTotal: netSalesFor(prevKey, prevKey) });
  }

  return {
    netSales: { value: netSales, previous: prevNetSales, changePct: changePct(netSales, prevNetSales), sparkline: spark },
    grossProfit: {
      value: gp.profit,
      previous: prevGp.profit,
      changePct: changePct(gp.profit, prevGp.profit),
      sparkline: spark,
      marginPct: gp.revenue > 0 ? round2((gp.profit / gp.revenue) * 100) : 0,
    },
    cash: { value: cash, previous: cashPrev, changePct: changePct(cash, cashPrev), sparkline: spark },
    receivables: {
      value: receivablesTotal,
      previous: receivablesPrev,
      changePct: changePct(receivablesTotal, receivablesPrev),
      sparkline: spark,
      overdue: receivablesOverdue(),
    },
    salesTrend: trend,
  };
});

/** Balance of a system-role account as of a given date (inclusive) — used for the KPI's "previous period" cash snapshot. */
function accountBalanceAsOf(role: SystemRole, asOf: string): number {
  const id = accountFor(role).id;
  let total = 0;
  for (const entry of db.journalEntries) {
    if (localDateKey(entry.date) > asOf) continue;
    for (const line of entry.lines) if (line.accountId === id) total += line.debit - line.credit;
  }
  return round2(total);
}

export interface TopProductRow {
  id: string;
  name: string;
  sku: string;
  qty: number;
  grossProfit: number;
}

export interface TopCustomerRow {
  id: string;
  name: string;
  total: number;
}

/** Top 5 products by gross profit (not revenue — docs/v2/11 Part B.5) for the given period. */
export const getTopProducts = wrap('core.getTopProducts', async function getTopProducts(period: HomePeriod = 'month', limit = 5): Promise<TopProductRow[]> {
  await delay();
  const { from, to } = periodRange(period);
  const byProduct = new Map<string, { qty: number; profit: number }>();
  for (const inv of invoicesInRange(from, to)) {
    for (const line of inv.lines) {
      const net = line.net ?? round2(line.qty * line.price - line.discount);
      const entry = byProduct.get(line.productId) ?? { qty: 0, profit: 0 };
      entry.qty += line.qty;
      entry.profit += net - line.qty * line.costPrice;
      byProduct.set(line.productId, entry);
    }
  }
  return [...byProduct.entries()]
    .map(([productId, v]) => {
      const p = db.products.find((x) => x.id === productId);
      return { id: productId, name: p?.name ?? '—', sku: p?.sku ?? '', qty: round2(v.qty), grossProfit: round2(v.profit) };
    })
    .sort((a, b) => b.grossProfit - a.grossProfit)
    .slice(0, limit);
});

/** Top 5 customers by net sales for the given period. */
export const getTopCustomers = wrap('core.getTopCustomers', async function getTopCustomers(period: HomePeriod = 'month', limit = 5): Promise<TopCustomerRow[]> {
  await delay();
  const { from, to } = periodRange(period);
  const byCustomer = new Map<string, number>();
  for (const inv of invoicesInRange(from, to)) {
    if (!inv.customerId) continue;
    byCustomer.set(inv.customerId, round2((byCustomer.get(inv.customerId) ?? 0) + inv.grandTotal));
  }
  return [...byCustomer.entries()]
    .map(([customerId, total]) => ({ id: customerId, name: db.customers.find((c) => c.id === customerId)?.name ?? '—', total }))
    .sort((a, b) => b.total - a.total)
    .slice(0, limit);
});

// =================================================================================================
// Notifications drawer reads (docs/v2/14-platform.md §6) — moved here from `useNotifications.ts`
// directly reading `db` (seam rule: controllers call services, not `src/mocks`).
// =================================================================================================

/** Stock transfers `SENT` (in transit) toward the given branch (or all branches when omitted). */
export function getInTransitTransfers(homeBranch: string | undefined): StockTransfer[] {
  return clone(db.stockTransfers.filter((t) => t.status === 'SENT' && (!homeBranch || t.toBranchId === homeBranch)));
}

/** Pending async approval requests. */
export function getPendingApprovalRequests(): ApprovalRequest[] {
  return clone(db.approvalRequests.filter((r) => r.status === 'pending'));
}

/** ISO timestamp of the last automatic-backup failure, if any. */
export function getLastBackupFailedAt(): string | undefined {
  return db.settings.backup?.lastBackupFailedAt;
}

/** Draft journal entries awaiting posting — used by the accountant home KPI. */
export function getJournalDraftCount(): number {
  return db.journalDrafts.length;
}

/** Current stock value (sum of active products' `stockValue`) — used by the storekeeper home KPI. */
export function getStockValueSnapshot(): number {
  return round2(db.products.filter((p) => p.active && p.type === 'product').reduce((a, p) => a + (p.stockValue ?? 0), 0));
}

/** True once at least one product exists — used to gate the storekeeper home's stock-value KPI loading state. */
export function hasAnyProducts(): boolean {
  return db.products.length > 0;
}

/** Subscribes to ledger-affecting mutations (journal postings, invoices, payments, …) — returns an unsubscribe function. */
export function onLedgerChanged(listener: () => void): () => void {
  return on('ledger:changed', listener);
}

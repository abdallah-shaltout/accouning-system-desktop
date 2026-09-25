import { ApiError, db, delay, inDateRange, localDateKey, round2, sum } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { customerStatement, supplierStatement } from '@/mocks/backend/balances';
import { getOpenDocumentsFor } from '@/mocks/backend/payments';
import type { Account, AccountKind } from '@/modules/accounting/types';
import { SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import { wrap } from '@/modules/diagnostics/services/defineService';

import type {
  AccountLedger,
  AgingReportRow,
  BalanceSheet,
  BranchComparisonRow,
  BusinessHealthReport,
  CashFlowStatement,
  DateRangeInput,
  DayBookEntry,
  DeadStockRow,
  DimensionFilter,
  DiscountReportRow,
  ExpensesReport,
  GrossProfitRow,
  InventoryReportRow,
  LowStockRow,
  OverdueRow,
  PeriodComparisonLine,
  ProfitAndLoss,
  ProfitLeakageReport,
  PurchasesReport,
  ReportRangeFilter,
  ReturnsReport,
  ReturnsReportRow,
  SalesReport,
  ShiftReportRow,
  StatementLine,
  StocktakeVarianceRow,
  TransferReportRow,
  TrialBalanceRow,
  VatCategoryBox,
  VatDetailRow,
  VatReport,
} from '../types';

/** Every report is computed on the fly from journal entries / documents — nothing is stored. */

function kindOf(account: Account): AccountKind {
  return account.kind;
}

/** The nearest header (isGroup) ancestor's name — what the trial balance shows as "groupName". */
function groupNameOf(account: Account): string {
  let parent = account.parentId ? db.accounts.find((a) => a.id === account.parentId) : undefined;
  while (parent && !parent.isGroup) parent = parent.parentId ? db.accounts.find((a) => a.id === parent!.parentId) : undefined;
  return parent?.name ?? '';
}

/**
 * Σ(debit − credit) per account for entries within the range. `dim` (v2 phase 12, docs/v2/13 §1
 * "branch, cost center, currency" filters) restricts to journal lines matching the given
 * branch/cost-center/currency — additive, optional, so every pre-existing caller (which omits it)
 * is byte-for-byte unchanged.
 */
function movements(range: DateRangeInput, dim?: DimensionFilter): Map<string, { d: number; c: number }> {
  const map = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    if (!inDateRange(e.date, range.from, range.to)) continue;
    for (const l of e.lines) {
      if (dim?.branchId && l.branchId !== dim.branchId) continue;
      if (dim?.costCenterId && l.costCenterId !== dim.costCenterId) continue;
      if (dim?.currency && l.currency !== dim.currency) continue;
      const t = map.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      map.set(l.accountId, t);
    }
  }
  return map;
}

function dayBefore(key: string): string {
  const [y, m, d] = key.split('-').map(Number);
  return localDateKey(new Date(y, m - 1, d - 1));
}

// --- Trial balance ---------------------------------------------------------------------------

export const getTrialBalance = wrap('reports.getTrialBalance', async function getTrialBalance(range: ReportRangeFilter): Promise<TrialBalanceRow[]> {
  await delay();
  const dim: DimensionFilter = { branchId: range.branchId, costCenterId: range.costCenterId, currency: range.currency };
  const opening = range.from ? movements({ to: dayBefore(range.from) }, dim) : new Map();
  const period = movements(range, dim);
  return db.accounts
    .filter((a) => !a.isGroup && (opening.has(a.id) || period.has(a.id)))
    .sort((a, b) => a.code.localeCompare(b.code))
    .map((a) => {
      const o = opening.get(a.id) ?? { d: 0, c: 0 };
      const p = period.get(a.id) ?? { d: 0, c: 0 };
      const closing = round2(o.d - o.c + p.d - p.c);
      return {
        accountId: a.id,
        code: a.code,
        name: a.name,
        groupName: groupNameOf(a),
        openingBalance: round2(o.d - o.c),
        periodDebit: round2(p.d),
        periodCredit: round2(p.c),
        closingDebit: closing > 0 ? closing : 0,
        closingCredit: closing < 0 ? -closing : 0,
      };
    });
});

// --- Profit & loss ---------------------------------------------------------------------------

function lines(kind: AccountKind, mv: Map<string, { d: number; c: number }>, sign: 1 | -1, filter?: (a: Account) => boolean): StatementLine[] {
  return db.accounts
    .filter((a) => !a.isGroup && kindOf(a) === kind && mv.has(a.id) && (!filter || filter(a)))
    .sort((a, b) => a.code.localeCompare(b.code))
    .map((a) => {
      const t = mv.get(a.id)!;
      return { accountId: a.id, code: a.code, name: a.name, amount: round2(sign * (t.d - t.c)) };
    })
    .filter((l) => Math.abs(l.amount) > 0.001);
}

function computePnl(range: DateRangeInput, dim?: DimensionFilter): ProfitAndLoss {
  const mv = movements(range, dim);
  const revenue = lines('REVENUE', mv, -1); // credit − debit → returns come out negative
  // costOfSales subtype (5xxx: COGS, inventory variance, write-offs, freight-in) vs 6xxx operating expenses.
  const cogs = lines('EXPENSE', mv, 1, (a) => a.subtype === 'costOfSales');
  const expenses = lines('EXPENSE', mv, 1, (a) => a.subtype !== 'costOfSales');
  const netRevenue = sum(revenue, (l) => l.amount);
  const totalCogs = sum(cogs, (l) => l.amount);
  const totalExpenses = sum(expenses, (l) => l.amount);
  const grossProfit = round2(netRevenue - totalCogs);
  return { revenue, netRevenue, cogs, totalCogs, grossProfit, expenses, totalExpenses, netIncome: round2(grossProfit - totalExpenses) };
}

export const getProfitAndLoss = wrap('reports.getProfitAndLoss', async function getProfitAndLoss(range: ReportRangeFilter): Promise<ProfitAndLoss> {
  await delay();
  return computePnl(range, { branchId: range.branchId, costCenterId: range.costCenterId, currency: range.currency });
});

/**
 * v2 phase 12 (docs/v2/13-reports.md §1 "comparison mode"): the same P&L for a second period, so
 * the page can show this-period vs previous-period/same-period-last-year side by side. Kept as a
 * thin wrapper (not baked into `getProfitAndLoss`) so every existing caller is unaffected.
 */
export const getProfitAndLossComparison = wrap('reports.getProfitAndLossComparison', async function getProfitAndLossComparison(range: ReportRangeFilter, compareRange: DateRangeInput): Promise<{ current: ProfitAndLoss; previous: ProfitAndLoss }> {
  await delay();
  const dim: DimensionFilter = { branchId: range.branchId, costCenterId: range.costCenterId, currency: range.currency };
  return { current: computePnl(range, dim), previous: computePnl(compareRange, dim) };
});

// ---------------------------------------------------------------------------------------------
// v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3): P&L by cost center + budget vs
// actual. Reuses `movements()`'s shape but keyed by (accountId, costCenterId) instead of just
// accountId, so every column below is computed from the exact same ledger lines the flat P&L reads
// — Σ columns always reconciles to `computePnl()`'s totals for the same range.
// ---------------------------------------------------------------------------------------------

const UNASSIGNED_CC = '__unassigned__';

function movementsByCostCenter(range: DateRangeInput): Map<string, Map<string, { d: number; c: number }>> {
  const byCc = new Map<string, Map<string, { d: number; c: number }>>();
  for (const e of db.journalEntries) {
    if (!inDateRange(e.date, range.from, range.to)) continue;
    for (const l of e.lines) {
      const ccId = l.costCenterId ?? UNASSIGNED_CC;
      const accMap = byCc.get(ccId) ?? new Map<string, { d: number; c: number }>();
      const t = accMap.get(l.accountId) ?? { d: 0, c: 0 };
      t.d += l.debit;
      t.c += l.credit;
      accMap.set(l.accountId, t);
      byCc.set(ccId, accMap);
    }
  }
  return byCc;
}

function pnlColumnFor(mv: Map<string, { d: number; c: number }>): Omit<import('../types').CostCenterPnlColumn, 'costCenterId' | 'name'> {
  const netRevenue = sum(lines('REVENUE', mv, -1), (l) => l.amount);
  const totalCogs = sum(lines('EXPENSE', mv, 1, (a) => a.subtype === 'costOfSales'), (l) => l.amount);
  const totalExpenses = sum(lines('EXPENSE', mv, 1, (a) => a.subtype !== 'costOfSales'), (l) => l.amount);
  const grossProfit = round2(netRevenue - totalCogs);
  return { netRevenue, totalCogs, grossProfit, totalExpenses, netIncome: round2(grossProfit - totalExpenses) };
}

export const getCostCenterProfitAndLoss = wrap('reports.getCostCenterProfitAndLoss', async function getCostCenterProfitAndLoss(range: DateRangeInput): Promise<import('../types').CostCenterPnl> {
  await delay();
  const byCc = movementsByCostCenter(range);
  const centers = db.costCenters
    .filter((c) => byCc.has(c.id))
    .map((c) => ({ costCenterId: c.id, name: c.name, ...pnlColumnFor(byCc.get(c.id)!) }))
    .sort((a, b) => b.netRevenue - a.netRevenue);
  const unassignedMv = byCc.get(UNASSIGNED_CC) ?? new Map();
  const unassigned = { costCenterId: UNASSIGNED_CC, name: 'غير مخصص', ...pnlColumnFor(unassignedMv) };
  const total = computePnl(range);
  return {
    centers,
    unassigned,
    total: { costCenterId: '__total__', name: 'الإجمالي', netRevenue: total.netRevenue, totalCogs: total.totalCogs, grossProfit: total.grossProfit, totalExpenses: total.totalExpenses, netIncome: total.netIncome },
  };
});

/** Budget vs actual (docs/v2/10 §3): actual = Σ expense-account movement for the cost center within the fiscal year's dates. */
export const getCostCenterBudgetVsActual = wrap('reports.getCostCenterBudgetVsActual', async function getCostCenterBudgetVsActual(fiscalYearId: string): Promise<import('../types').CostCenterBudgetRow[]> {
  await delay();
  const fy = db.fiscalYears.find((f) => f.id === fiscalYearId);
  if (!fy) throw new ApiError('السنة المالية غير موجودة', 'NOT_FOUND');
  const byCc = movementsByCostCenter({ from: fy.startDate, to: fy.endDate });
  const rows: import('../types').CostCenterBudgetRow[] = [];
  for (const c of db.costCenters) {
    const budgetRow = c.budgets?.find((b) => b.fiscalYearId === fiscalYearId);
    if (!budgetRow) continue;
    const mv = byCc.get(c.id) ?? new Map();
    const actual = sum(lines('EXPENSE', mv, 1), (l) => l.amount);
    const variancePct = budgetRow.amount > 0 ? round2(((actual - budgetRow.amount) / budgetRow.amount) * 100) : 0;
    // Phase 10: also surfaced as a real dismiss/snooze-able insight ("budget" rule in
    // `modules/core/services/insightRules.ts`, docs/v2/11 D2) — `nearBudget` below stays as the
    // plain boolean column this report itself renders.
    rows.push({ costCenterId: c.id, name: c.name, budget: budgetRow.amount, actual, variancePct, nearBudget: budgetRow.amount > 0 && actual >= budgetRow.amount * 0.9 });
  }
  return rows.sort((a, b) => b.actual - a.actual);
});

// --- Balance sheet ---------------------------------------------------------------------------

export const getBalanceSheet = wrap('reports.getBalanceSheet', async function getBalanceSheet(asOf: string, dim?: DimensionFilter): Promise<BalanceSheet> {
  await delay();
  const mv = movements({ to: asOf }, dim);
  const assets = lines('ASSET', mv, 1);
  const liabilities = lines('LIABILITY', mv, -1);
  const equity = lines('EQUITY', mv, -1);
  const unclosedEarnings = computePnl({ to: asOf }, dim).netIncome;
  const totalAssets = sum(assets, (l) => l.amount);
  const totalLiabilities = sum(liabilities, (l) => l.amount);
  const totalEquity = round2(sum(equity, (l) => l.amount) + unclosedEarnings);
  return {
    asOf,
    assets,
    totalAssets,
    liabilities,
    totalLiabilities,
    equity,
    unclosedEarnings,
    totalEquity,
    balanced: Math.abs(totalAssets - totalLiabilities - totalEquity) < 0.01,
  };
});

// --- Ledgers / statements ---------------------------------------------------------------------

export const getAccountLedger = wrap('reports.getAccountLedger', async function getAccountLedger(accountId: string, range: DateRangeInput): Promise<AccountLedger> {
  await delay();
  const account = db.accounts.find((a) => a.id === accountId);
  if (!account) throw new ApiError('الحساب غير موجود', 'NOT_FOUND');
  const sign = account.normalSide === 'DEBIT' ? 1 : -1;
  let opening = 0;
  const rows: AccountLedger['rows'] = [];
  const rowLinks: Record<string, string> = {};
  const entries = [...db.journalEntries].sort((a, b) => a.date.localeCompare(b.date) || a.number.localeCompare(b.number));
  for (const e of entries) {
    for (const l of e.lines) {
      if (l.accountId !== accountId) continue;
      if (range.from && localDateKey(e.date) < range.from) {
        opening += sign * (l.debit - l.credit);
        continue;
      }
      if (range.to && localDateKey(e.date) > range.to) continue;
      rows.push({ id: l.id, date: e.date, entryId: e.id, entryNumber: e.number, description: l.description ?? e.description, debit: l.debit, credit: l.credit, balance: 0 });
      rowLinks[l.id] = `/accounting/journal/${e.id}`;
    }
  }
  let running = round2(opening);
  for (const r of rows) {
    running = round2(running + sign * (r.debit - r.credit));
    r.balance = running;
  }
  return {
    title: `${account.code} — ${account.name}`,
    subtitle: account.normalSide === 'DEBIT' ? 'حساب مدين الطبيعة' : 'حساب دائن الطبيعة',
    normalSide: account.normalSide,
    openingBalance: round2(opening),
    rows,
    totalDebit: sum(rows, (r) => r.debit),
    totalCredit: sum(rows, (r) => r.credit),
    closingBalance: running,
    rowLinks,
  };
});

/** Customer / supplier statement in the same shape as an account ledger. */
export const getPartyLedger = wrap('reports.getPartyLedger', async function getPartyLedger(kind: 'customer' | 'supplier', partyId: string, range: DateRangeInput): Promise<AccountLedger> {
  await delay();
  const party = (kind === 'customer' ? db.customers : db.suppliers).find((p) => p.id === partyId);
  if (!party) throw new ApiError(kind === 'customer' ? 'العميل غير موجود' : 'المورد غير موجود', 'NOT_FOUND');
  const all = kind === 'customer' ? customerStatement(partyId) : supplierStatement(partyId);
  const before = all.filter((r) => range.from && localDateKey(r.date) < range.from);
  const inRange = all.filter((r) => inDateRange(r.date, range.from, range.to));
  const opening = before.at(-1)?.balance ?? 0;
  const rowLinks: Record<string, string> = {};
  for (const r of inRange) {
    rowLinks[r.id] = r.kind === 'invoice' || r.kind === 'refund' ? `/invoices/${r.refId}` : r.kind === 'payment' ? `/payments?highlight=${r.refId}` : `/purchases/${r.refId}`;
  }
  return {
    title: party.name,
    subtitle: kind === 'customer' ? 'كشف حساب عميل — الرصيد الموجب مستحق لنا' : 'كشف حساب مورد — الرصيد الموجب مستحق للمورد',
    normalSide: kind === 'customer' ? 'DEBIT' : 'CREDIT',
    openingBalance: opening,
    rows: inRange.map((r) => ({ id: r.id, date: r.date, entryId: r.refId, entryNumber: r.number, description: r.description, debit: r.debit, credit: r.credit, balance: r.balance })),
    totalDebit: sum(inRange, (r) => r.debit),
    totalCredit: sum(inRange, (r) => r.credit),
    closingBalance: inRange.at(-1)?.balance ?? opening,
    rowLinks,
  };
});

// --- Sales -----------------------------------------------------------------------------------

export const getSalesReport = wrap('reports.getSalesReport', async function getSalesReport(range: DateRangeInput): Promise<SalesReport> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));

  const byProduct = new Map<string, SalesReport['byProduct'][number]>();
  for (const inv of invoices) {
    const factor = 1 - inv.discountRate / 100;
    // Tax-inclusive invoices (the default, docs/v2/06 §3) carry VAT inside `subTotal`/line prices:
    // there `subTotal − discount = grandTotal`. Strip each line's VAT so "revenue" is pre-tax.
    const inclusive = inv.taxAmount > 0 && Math.abs(inv.subTotal - inv.discountAmount - inv.grandTotal) < 0.01;
    for (const l of inv.lines) {
      const row = byProduct.get(l.productId) ?? { productId: l.productId, name: l.name, qty: 0, revenue: 0, cost: 0, profit: 0 };
      const product = db.products.find((p) => p.id === l.productId);
      row.qty += l.qty;
      row.revenue += ((l.qty * l.price - l.discount) * factor) / (inclusive ? 1 + (l.taxRate ?? inv.taxRate) / 100 : 1);
      row.cost += product?.type === 'product' ? l.qty * l.costPrice : 0;
      byProduct.set(l.productId, row);
    }
  }
  const productRows = [...byProduct.values()]
    .map((r) => ({ ...r, revenue: round2(r.revenue), cost: round2(r.cost), profit: round2(r.revenue - r.cost) }))
    .sort((a, b) => b.revenue - a.revenue);

  const byCategory = new Map<string, { name: string; qty: number; revenue: number }>();
  for (const r of productRows) {
    const catId = db.products.find((p) => p.id === r.productId)?.categoryId;
    const name = db.categories.find((c) => c.id === catId)?.name ?? 'بدون تصنيف';
    const row = byCategory.get(name) ?? { name, qty: 0, revenue: 0 };
    row.qty += r.qty;
    row.revenue = round2(row.revenue + r.revenue);
    byCategory.set(name, row);
  }

  const days = new Map<string, { date: string; invoices: number; total: number }>();
  for (const inv of invoices) {
    const key = localDateKey(inv.date);
    const row = days.get(key) ?? { date: key, invoices: 0, total: 0 };
    row.invoices += 1;
    row.total = round2(row.total + inv.grandTotal);
    days.set(key, row);
  }

  const group = <K extends string>(keyOf: (i: (typeof invoices)[number]) => K, label: (k: K) => string) => {
    const map = new Map<K, { count: number; total: number }>();
    for (const inv of invoices) {
      const k = keyOf(inv);
      const row = map.get(k) ?? { count: 0, total: 0 };
      row.count += 1;
      row.total = round2(row.total + inv.grandTotal);
      map.set(k, row);
    }
    return [...map.entries()].map(([k, v]) => ({ key: label(k), ...v })).sort((a, b) => b.total - a.total);
  };

  const grossSales = sum(invoices, (i) => i.subTotal);
  const discounts = sum(invoices, (i) => i.discountAmount);
  const vat = sum(invoices, (i) => i.taxAmount);
  const total = sum(invoices, (i) => i.grandTotal);
  // Pre-tax net sales = what the customer paid minus VAT — correct for tax-inclusive and
  // tax-exclusive invoices alike (`subTotal − discount` is VAT-inclusive on inclusive invoices).
  const netSales = round2(total - vat);
  const refundTotal = sum(refunds, (r) => r.grandTotal);
  const cogs = sum(productRows, (r) => r.cost);

  return {
    summary: {
      invoiceCount: invoices.length,
      grossSales,
      discounts,
      netSales,
      vat,
      total,
      refunds: refundTotal,
      netAfterRefunds: round2(total - refundTotal),
      cogs,
      grossProfit: round2(netSales - cogs),
      averageInvoice: invoices.length ? round2(total / invoices.length) : 0,
    },
    byDay: [...days.values()].sort((a, b) => a.date.localeCompare(b.date)),
    byProduct: productRows,
    byCategory: [...byCategory.values()].sort((a, b) => b.revenue - a.revenue),
    byMethod: group((i) => i.paymentMethod, (k) => SALE_METHOD_LABEL[k]).map(({ key, ...v }) => ({ method: key, ...v })),
    byCashier: group((i) => i.cashierId, (k) => db.users.find((u) => u.id === k)?.name ?? '—').map(({ key, ...v }) => ({ name: key, ...v })),
  };
});

// --- Inventory -------------------------------------------------------------------------------

export const getInventoryReport = wrap('reports.getInventoryReport', async function getInventoryReport(): Promise<InventoryReportRow[]> {
  await delay();
  return db.products
    .filter((p) => p.type === 'product' && p.active)
    .map((p): InventoryReportRow => ({
      productId: p.id,
      name: p.name,
      sku: p.sku,
      category: db.categories.find((c) => c.id === p.categoryId)?.name ?? 'بدون تصنيف',
      qty: p.stockQty,
      minStock: p.minStock ?? 0,
      costPrice: p.costPrice,
      price: p.price,
      costValue: round2(p.stockQty * p.costPrice),
      retailValue: round2(p.stockQty * p.price),
      status: p.stockQty <= 0 ? 'out' : p.stockQty <= (p.minStock ?? 0) ? 'low' : 'ok',
    }))
    .sort((a, b) => a.category.localeCompare(b.category, 'ar') || a.name.localeCompare(b.name, 'ar'));
});

// --- VAT -------------------------------------------------------------------------------------

/**
 * v2 (docs/v2/02-accounting-review.md §4 invariant 5): output VAT for a period = Σ VAT on invoice
 * LINES − Σ VAT on credit-note lines (not the invoice-level `taxAmount` snapshot alone, now that
 * VAT is computed per line with its own category — docs/v2/06-sales-and-pos.md §3). Grouped by
 * (category, rate) so zero-rated and exempt land in their own ZATCA return boxes (D1).
 *
 * Credit notes (`Refund`) don't carry a per-line category breakdown (that's the invoice's own
 * `lines[].taxCategory` — a return only ever returns quantities of existing invoice lines), so a
 * refund's VAT is apportioned back into the *same* invoice's category mix, in proportion to each
 * category's share of that invoice's total VAT. This keeps `Σ salesBoxes[].vat === outputVat` exact
 * even when a return spans lines from more than one tax category.
 */
function salesVatBoxes(invoices: typeof db.invoices, refunds: typeof db.refunds): { boxes: VatCategoryBox[]; vat: number; taxable: number } {
  const boxes = new Map<string, VatCategoryBox>();
  const add = (category: 'S' | 'Z' | 'E' | 'O', rate: number, net: number, vat: number, countDelta: number) => {
    const key = `${category}:${rate}`;
    const box = boxes.get(key) ?? { category, rate, net: 0, vat: 0, count: 0 };
    box.net = round2(box.net + net);
    box.vat = round2(box.vat + vat);
    box.count += countDelta;
    boxes.set(key, box);
  };

  // `count` per box is document-level noise the report doesn't currently surface per box (the top-
  // level `sales.count`/`salesReturns.count` cover that) — left at 0 here on purpose.
  for (const inv of invoices) {
    for (const line of inv.lines) {
      add(line.taxCategory ?? 'O', line.taxRate ?? 0, line.net ?? 0, line.vat ?? 0, 0);
    }
  }

  for (const refund of refunds) {
    const inv = db.invoices.find((i) => i.id === refund.invoiceId);
    if (!inv || !inv.taxAmount) continue;
    // Apportion this refund's VAT across the invoice's category mix, in proportion to each
    // category's share of the invoice's total VAT — a return has no per-line category of its own.
    const invVat = inv.taxAmount;
    const invNet = inv.subTotal - inv.discountAmount;
    for (const line of inv.lines) {
      const lineVat = line.vat ?? 0;
      const lineNet = line.net ?? 0;
      const shareOfVat = invVat > 0 ? lineVat / invVat : 0;
      const shareOfNet = invNet > 0 ? lineNet / invNet : 0;
      add(line.taxCategory ?? 'O', line.taxRate ?? 0, -round2(refund.subTotal * shareOfNet), -round2(refund.taxAmount * shareOfVat), 0);
    }
  }

  const list = [...boxes.values()].filter((b) => b.net !== 0 || b.vat !== 0);
  return { boxes: list, vat: round2(list.reduce((a, b) => a + b.vat, 0)), taxable: round2(list.reduce((a, b) => a + b.net, 0)) };
}

/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §2): every VAT report figure below is
 * compared against the (always base-currency) vatOutput/vatInput ledger, so an FC invoice's
 * `taxAmount`/`subTotal`/`discountAmount`/lines' `net`/`vat` — all in the invoice's OWN currency —
 * must be converted to base first, at the invoice's own `exchangeRate`, exactly like every other
 * base-currency reader in the app. Refunds/POs/expenses aren't extended to FC in this phase (see
 * the phase report's deviations), so only invoices need this.
 */
function toBaseInvoice(inv: (typeof db.invoices)[number]): (typeof db.invoices)[number] {
  if (!inv.currency || !inv.exchangeRate) return inv;
  const rate = inv.exchangeRate;
  return {
    ...inv,
    subTotal: round2(inv.subTotal * rate),
    discountAmount: round2(inv.discountAmount * rate),
    taxAmount: round2(inv.taxAmount * rate),
    grandTotal: round2(inv.grandTotal * rate),
    lines: inv.lines.map((l) => ({ ...l, net: l.net !== undefined ? round2(l.net * rate) : l.net, vat: l.vat !== undefined ? round2(l.vat * rate) : l.vat })),
  };
}

export const getVatReport = wrap('reports.getVatReport', async function getVatReport(range: DateRangeInput): Promise<VatReport> {
  await delay();
  const invoices = db.invoices.filter((i) => inDateRange(i.date, range.from, range.to)).map(toBaseInvoice);
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));
  // v2 phase 8 (docs/v2/09-purchases-payments-expenses.md §1 "VAT", review E3): a non-VAT
  // supplier's receipt never claims input VAT — it's added to cost instead — so it (and its debit
  // notes) are excluded here, same as `scripts/verify/sales.ts`'s invariant. Tax-invoice expenses
  // (§4) also claim input VAT and are added in so this reconciles with the vatInput ledger.
  const pos = db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && !p.vatNotRecoverable && inDateRange(p.date, range.from, range.to));
  const recoverablePoIds = new Set(db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && !p.vatNotRecoverable).map((p) => p.id));
  const pReturns = db.purchaseReturns.filter((r) => recoverablePoIds.has(r.purchaseOrderId) && inDateRange(r.date, range.from, range.to));
  const expenses = db.expenses.filter((e) => e.isTaxInvoice && inDateRange(e.date, range.from, range.to));

  const salesBoxData = salesVatBoxes(invoices, refunds);
  const sales = { taxable: sum(invoices, (i) => i.subTotal - i.discountAmount), vat: sum(invoices, (i) => i.taxAmount), count: invoices.length };
  const salesReturns = { taxable: sum(refunds, (r) => r.subTotal), vat: sum(refunds, (r) => r.taxAmount), count: refunds.length };
  const purchases = {
    taxable: round2(sum(pos, (p) => p.subTotal) + sum(expenses, (e) => e.netAmount)),
    vat: round2(sum(pos, (p) => p.taxAmount) + sum(expenses, (e) => e.taxAmount)),
    count: pos.length + expenses.length,
  };
  const purchaseReturns = { taxable: sum(pReturns, (r) => r.subTotal), vat: sum(pReturns, (r) => r.taxAmount), count: pReturns.length };
  const outputVat = round2(sales.vat - salesReturns.vat);
  const inputVat = round2(purchases.vat - purchaseReturns.vat);

  const mv = movements(range);
  const out = mv.get(accountFor('vatOutput').id) ?? { d: 0, c: 0 };
  const inp = mv.get(accountFor('vatInput').id) ?? { d: 0, c: 0 };

  return {
    sales,
    salesReturns,
    purchases,
    purchaseReturns,
    outputVat,
    inputVat,
    netPayable: round2(outputVat - inputVat),
    ledgerOutput: round2(out.c - out.d),
    ledgerInput: round2(inp.d - inp.c),
    salesBoxes: salesBoxData.boxes,
  };
});

/**
 * v2 phase 12 (docs/v2/13 §2 "VAT detail: line-level listing per document with category, net, VAT
 * — the audit trail behind each box"). Same source rows `salesVatBoxes()` aggregates, kept flat.
 */
export const getVatDetail = wrap('reports.getVatDetail', async function getVatDetail(range: DateRangeInput): Promise<VatDetailRow[]> {
  await delay();
  const invoices = db.invoices.filter((i) => inDateRange(i.date, range.from, range.to)).map(toBaseInvoice);
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));
  const rows: VatDetailRow[] = [];
  for (const inv of invoices) {
    for (const line of inv.lines) {
      if (!line.net && !line.vat) continue;
      rows.push({
        id: `${inv.id}:${line.id}`,
        date: inv.date,
        documentNumber: inv.number,
        documentKind: 'sale',
        productName: line.name,
        category: line.taxCategory ?? 'O',
        rate: line.taxRate ?? 0,
        net: line.net ?? 0,
        vat: line.vat ?? 0,
      });
    }
  }
  for (const refund of refunds) {
    const inv = db.invoices.find((i) => i.id === refund.invoiceId);
    rows.push({
      id: refund.id,
      date: refund.date,
      documentNumber: refund.number,
      documentKind: 'salesReturn',
      productName: inv ? `مرتجع — ${inv.number}` : 'مرتجع',
      category: 'S',
      rate: inv?.taxRate ?? 0,
      net: -refund.subTotal,
      vat: -refund.taxAmount,
    });
  }
  return rows.sort((a, b) => b.date.localeCompare(a.date));
});

// --- Lookups for report filters ---------------------------------------------------------------

export const getLedgerTargets = wrap('reports.getLedgerTargets', async function getLedgerTargets(): Promise<{ accounts: { id: string; label: string }[]; customers: { id: string; label: string }[]; suppliers: { id: string; label: string }[] }> {
  await delay(80);
  return {
    accounts: [...db.accounts].sort((a, b) => a.code.localeCompare(b.code)).map((a) => ({ id: a.id, label: `${a.code} — ${a.name}` })),
    customers: db.customers.map((c) => ({ id: c.id, label: c.name })),
    suppliers: db.suppliers.map((s) => ({ id: s.id, label: s.name })),
  };
});

// ===============================================================================================
// v2 phase 12 (docs/v2/13-reports.md §2) — new reports. Everything below reads existing db tables
// only; no posting/mutating logic is touched.
// ===============================================================================================

/** Small helper: every cash/bank-subtype ASSET account (docs/v2/03 §3's statement classification). */
function cashLikeAccountIds(): Set<string> {
  return new Set(db.accounts.filter((a) => !a.isGroup && a.kind === 'ASSET' && (a.subtype === 'cash' || a.subtype === 'bank')).map((a) => a.id));
}

// --- Cash flow statement (indirect method) ------------------------------------------------------

export const getCashFlowStatement = wrap('reports.getCashFlowStatement', async function getCashFlowStatement(range: DateRangeInput): Promise<CashFlowStatement> {
  await delay();
  const pnl = computePnl(range);
  const cashIds = cashLikeAccountIds();
  const openingMv = range.from ? movements({ to: dayBefore(range.from) }) : new Map();
  const closingMv = movements({ to: range.to ?? localDateKey(new Date()) });
  const sumCash = (mv: Map<string, { d: number; c: number }>) => [...cashIds].reduce((a, id) => a + ((mv.get(id)?.d ?? 0) - (mv.get(id)?.c ?? 0)), 0);
  const openingCash = round2(sumCash(openingMv));
  const closingCash = round2(sumCash(closingMv));

  // Working-capital changes: Δ(receivables) and Δ(payables/inventory) over the period, from the
  // control/inventory accounts' own net movement (non-cash accounts only).
  const periodMv = movements(range);
  const wcAccount = (subtype: import('@/modules/accounting/types').AccountSubtype, sign: 1 | -1) => {
    const acc = db.accounts.find((a) => !a.isGroup && a.subtype === subtype);
    if (!acc) return 0;
    const t = periodMv.get(acc.id) ?? { d: 0, c: 0 };
    return round2(sign * (t.d - t.c));
  };
  const operatingAdjustments = [
    { label: 'التغير في الذمم المدينة (العملاء)', amount: -wcAccount('receivable', 1) },
    { label: 'التغير في المخزون', amount: -wcAccount('inventory', 1) },
    { label: 'التغير في الذمم الدائنة (الموردون)', amount: wcAccount('payable', -1) },
  ].filter((l) => Math.abs(l.amount) > 0.001);
  const operatingCash = round2(pnl.netIncome + sum(operatingAdjustments, (l) => l.amount));

  // Investing: fixed-asset account movement (purchases negative, disposals positive).
  const fixedAssetIds = db.accounts.filter((a) => !a.isGroup && a.kind === 'ASSET' && a.subtype === 'fixedAsset').map((a) => a.id);
  const investing = fixedAssetIds
    .map((id) => {
      const acc = db.accounts.find((a) => a.id === id)!;
      const t = periodMv.get(id) ?? { d: 0, c: 0 };
      return { label: acc.name, amount: round2(-(t.d - t.c)) };
    })
    .filter((l) => Math.abs(l.amount) > 0.001);
  const investingCash = round2(sum(investing, (l) => l.amount));

  // Financing: capital, loans (long-term liability), drawings — EQUITY accounts (excluding retained/current
  // earnings, which the P&L already covers) plus LIABILITY accounts classified as long-term (loans).
  const financingAccounts = db.accounts.filter(
    (a) => !a.isGroup && ((a.kind === 'EQUITY' && a.systemRole !== 'retainedEarnings' && a.systemRole !== 'currentEarnings') || (a.kind === 'LIABILITY' && a.subtype === 'longTermLiability')),
  );
  const financing = financingAccounts
    .map((a) => {
      const t = periodMv.get(a.id) ?? { d: 0, c: 0 };
      return { label: a.name, amount: round2(t.c - t.d) };
    })
    .filter((l) => Math.abs(l.amount) > 0.001);
  const financingCash = round2(sum(financing, (l) => l.amount));

  const netChange = round2(operatingCash + investingCash + financingCash);
  return { netIncome: pnl.netIncome, operatingAdjustments, operatingCash, investing, investingCash, financing, financingCash, netChange, openingCash, closingCash };
});

// --- Day book (دفتر اليومية) --------------------------------------------------------------------

export const getDayBook = wrap('reports.getDayBook', async function getDayBook(range: DateRangeInput): Promise<DayBookEntry[]> {
  await delay();
  return db.journalEntries
    .filter((e) => inDateRange(e.date, range.from, range.to))
    .sort((a, b) => a.date.localeCompare(b.date) || a.number.localeCompare(b.number))
    .map((e) => ({
      id: e.id,
      number: e.number,
      date: e.date,
      description: e.description,
      lines: e.lines.map((l) => {
        const acc = db.accounts.find((a) => a.id === l.accountId);
        return { accountCode: acc?.code ?? '', accountName: acc?.name ?? '', debit: l.debit, credit: l.credit };
      }),
    }));
});

// --- AR / AP aging --------------------------------------------------------------------------

/** v2 phase 12: the per-party aging (docs/v2/13 §2 "AR aging / AP aging"), across every party with an open balance. Reuses `getOpenDocumentsFor` (Phase 4's per-party aging on the party page) so bucket math stays identical. */
export const getAgingReport = wrap('reports.getAgingReport', async function getAgingReport(kind: 'customer' | 'supplier'): Promise<AgingReportRow[]> {
  await delay();
  const today = localDateKey(new Date());
  const parties = kind === 'customer' ? db.customers : db.suppliers;
  const rows: AgingReportRow[] = [];
  for (const party of parties) {
    const docs = getOpenDocumentsFor(kind, party.id);
    if (!docs.length) continue;
    const row: AgingReportRow = { partyId: party.id, name: party.name, current: 0, b30: 0, b60: 0, b90plus: 0, total: 0 };
    for (const doc of docs) {
      const refDate = doc.dueDate ?? doc.date;
      const daysOverdue = Math.floor((new Date(today).getTime() - new Date(localDateKey(refDate)).getTime()) / 86_400_000);
      if (daysOverdue <= 0) row.current = round2(row.current + doc.outstanding);
      else if (daysOverdue <= 30) row.b30 = round2(row.b30 + doc.outstanding);
      else if (daysOverdue <= 60) row.b60 = round2(row.b60 + doc.outstanding);
      else row.b90plus = round2(row.b90plus + doc.outstanding);
    }
    row.total = round2(row.current + row.b30 + row.b60 + row.b90plus);
    if (row.total > 0.001) rows.push(row);
  }
  return rows.sort((a, b) => b.total - a.total);
});

// --- Overdue invoices ------------------------------------------------------------------------

export const getOverdueReport = wrap('reports.getOverdueReport', async function getOverdueReport(kind: 'customer' | 'supplier'): Promise<OverdueRow[]> {
  await delay();
  const today = localDateKey(new Date());
  const parties = kind === 'customer' ? db.customers : db.suppliers;
  const rows: OverdueRow[] = [];
  for (const party of parties) {
    for (const doc of getOpenDocumentsFor(kind, party.id)) {
      if (!doc.dueDate) continue;
      const daysOverdue = Math.floor((new Date(today).getTime() - new Date(localDateKey(doc.dueDate)).getTime()) / 86_400_000);
      if (daysOverdue <= 0) continue;
      rows.push({
        id: doc.id,
        kind: doc.kind === 'invoice' ? 'invoice' : 'purchaseOrder',
        number: doc.number,
        partyId: party.id,
        partyName: party.name,
        phone: (party as { phone?: string }).phone,
        date: doc.date,
        dueDate: doc.dueDate,
        daysOverdue,
        outstanding: doc.outstanding,
      });
    }
  }
  return rows.sort((a, b) => b.daysOverdue - a.daysOverdue);
});

// --- Gross profit ---------------------------------------------------------------------------

function marginPct(revenue: number, profit: number): number {
  return revenue > 0 ? round2((profit / revenue) * 100) : 0;
}

export const getGrossProfitReport = wrap('reports.getGrossProfitReport', async function getGrossProfitReport(range: DateRangeInput, groupBy: 'invoice' | 'product' | 'category'): Promise<GrossProfitRow[]> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const map = new Map<string, { label: string; qty: number; revenue: number; cost: number }>();
  for (const inv of invoices) {
    const factor = 1 - inv.discountRate / 100;
    if (groupBy === 'invoice') {
      const revenue = round2(inv.subTotal - inv.discountAmount);
      const cost = sum(inv.lines, (l) => (db.products.find((p) => p.id === l.productId)?.type === 'product' ? l.qty * l.costPrice : 0));
      map.set(inv.id, { label: inv.number, qty: sum(inv.lines, (l) => l.qty), revenue, cost: round2(cost) });
      continue;
    }
    for (const l of inv.lines) {
      const product = db.products.find((p) => p.id === l.productId);
      const key = groupBy === 'product' ? l.productId : (db.categories.find((c) => c.id === product?.categoryId)?.name ?? 'بدون تصنيف');
      const label = groupBy === 'product' ? l.name : key;
      const row = map.get(key) ?? { label, qty: 0, revenue: 0, cost: 0 };
      row.qty += l.qty;
      row.revenue = round2(row.revenue + (l.qty * l.price - l.discount) * factor);
      row.cost = round2(row.cost + (product?.type === 'product' ? l.qty * l.costPrice : 0));
      map.set(key, row);
    }
  }
  return [...map.entries()]
    .map(([key, r]) => ({ key, label: r.label, qty: round2(r.qty), revenue: r.revenue, cost: r.cost, profit: round2(r.revenue - r.cost), marginPct: marginPct(r.revenue, r.revenue - r.cost) }))
    .sort((a, b) => b.profit - a.profit);
});

// --- Returns analysis ------------------------------------------------------------------------

export const getReturnsReport = wrap('reports.getReturnsReport', async function getReturnsReport(range: DateRangeInput): Promise<ReturnsReport> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));

  const groupCount = <K extends string>(keyOf: (r: (typeof refunds)[number]) => K, labelOf: (k: K) => string) => {
    const map = new Map<K, { count: number; qty: number; amount: number }>();
    for (const r of refunds) {
      const k = keyOf(r);
      const row = map.get(k) ?? { count: 0, qty: 0, amount: 0 };
      row.count += 1;
      row.qty += sum(r.lines, (l) => l.qty);
      row.amount = round2(row.amount + r.grandTotal);
      map.set(k, row);
    }
    return [...map.entries()].map(([k, v]) => ({ key: k, label: labelOf(k), ...v })).sort((a, b) => b.amount - a.amount);
  };

  const byReason: ReturnsReportRow[] = groupCount((r) => (r.reason ?? 'غير محدد') as string, (k) => k);

  const productMap = new Map<string, { count: number; qty: number; amount: number }>();
  for (const r of refunds) {
    const inv = db.invoices.find((i) => i.id === r.invoiceId);
    if (!inv) continue;
    for (const rl of r.lines) {
      const invLine = inv.lines.find((l) => l.id === rl.invoiceLineId);
      if (!invLine) continue;
      const row = productMap.get(invLine.productId) ?? { count: 0, qty: 0, amount: 0 };
      row.count += 1;
      row.qty += rl.qty;
      row.amount = round2(row.amount + rl.qty * invLine.price);
      productMap.set(invLine.productId, row);
    }
  }
  const byProduct: ReturnsReportRow[] = [...productMap.entries()]
    .map(([id, v]) => ({ key: id, label: db.products.find((p) => p.id === id)?.name ?? id, ...v }))
    .sort((a, b) => b.amount - a.amount);

  const cashierMap = new Map<string, { count: number; qty: number; amount: number }>();
  for (const r of refunds) {
    const inv = db.invoices.find((i) => i.id === r.invoiceId);
    const cashierId = inv?.cashierId ?? '—';
    const row = cashierMap.get(cashierId) ?? { count: 0, qty: 0, amount: 0 };
    row.count += 1;
    row.qty += sum(r.lines, (l) => l.qty);
    row.amount = round2(row.amount + r.grandTotal);
    cashierMap.set(cashierId, row);
  }
  const byCashier: ReturnsReportRow[] = [...cashierMap.entries()]
    .map(([id, v]) => ({ key: id, label: db.users.find((u) => u.id === id)?.name ?? id, ...v }))
    .sort((a, b) => b.amount - a.amount);

  const totalInvoices = invoices.length;
  const totalRefunds = refunds.length;
  return { totalInvoices, totalRefunds, returnRatePct: totalInvoices > 0 ? round2((totalRefunds / totalInvoices) * 100) : 0, byReason, byProduct, byCashier };
});

// --- Discounts & price overrides --------------------------------------------------------------

export const getDiscountsReport = wrap('reports.getDiscountsReport', async function getDiscountsReport(range: DateRangeInput, groupBy: 'cashier' | 'product'): Promise<DiscountReportRow[]> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const map = new Map<string, { label: string; invoiceCount: number; listValue: number; chargedValue: number }>();
  if (groupBy === 'cashier') {
    for (const inv of invoices) {
      const label = db.users.find((u) => u.id === inv.cashierId)?.name ?? inv.cashierId;
      const row = map.get(inv.cashierId) ?? { label, invoiceCount: 0, listValue: 0, chargedValue: 0 };
      row.invoiceCount += 1;
      const listValue = sum(inv.lines, (l) => l.qty * l.price);
      const chargedValue = round2(listValue - inv.discountAmount - sum(inv.lines, (l) => l.discount));
      row.listValue = round2(row.listValue + listValue);
      row.chargedValue = round2(row.chargedValue + chargedValue);
      map.set(inv.cashierId, row);
    }
  } else {
    for (const inv of invoices) {
      for (const l of inv.lines) {
        const row = map.get(l.productId) ?? { label: l.name, invoiceCount: 0, listValue: 0, chargedValue: 0 };
        row.invoiceCount += 1;
        const listValue = round2(l.qty * l.price);
        const chargedValue = round2(listValue - l.discount);
        row.listValue = round2(row.listValue + listValue);
        row.chargedValue = round2(row.chargedValue + chargedValue);
        map.set(l.productId, row);
      }
    }
  }
  return [...map.entries()]
    .map(([key, r]) => ({
      key,
      label: r.label,
      invoiceCount: r.invoiceCount,
      listValue: r.listValue,
      chargedValue: r.chargedValue,
      discountValue: round2(r.listValue - r.chargedValue),
      discountPct: r.listValue > 0 ? round2(((r.listValue - r.chargedValue) / r.listValue) * 100) : 0,
    }))
    .filter((r) => r.discountValue > 0.001)
    .sort((a, b) => b.discountValue - a.discountValue);
});

// --- Shifts / Z-report history -----------------------------------------------------------------

export const getShiftsReport = wrap('reports.getShiftsReport', async function getShiftsReport(range: DateRangeInput): Promise<ShiftReportRow[]> {
  await delay();
  return db.shifts
    .filter((s) => s.status === 'CLOSED' && inDateRange(s.openedAt, range.from, range.to))
    .map((s) => {
      const salesTotal = sum(
        db.invoices.filter((i) => i.status !== 'DRAFT' && i.cashierId === s.openedBy && inDateRange(i.date, s.openedAt, s.closedAt ?? s.openedAt)),
        (i) => i.grandTotal,
      );
      return {
        id: s.id,
        number: s.number,
        terminalId: s.terminalId,
        openedAt: s.openedAt,
        closedAt: s.closedAt,
        openedBy: db.users.find((u) => u.id === s.openedBy)?.name ?? s.openedBy,
        closedBy: s.closedBy ? (db.users.find((u) => u.id === s.closedBy)?.name ?? s.closedBy) : undefined,
        expectedCash: s.expectedCash ?? 0,
        countedCash: s.countedCash ?? 0,
        variance: s.variance ?? 0,
        salesTotal,
      };
    })
    .sort((a, b) => b.openedAt.localeCompare(a.openedAt));
});

// --- Low / dead stock ------------------------------------------------------------------------

export const getLowStockReport = wrap('reports.getLowStockReport', async function getLowStockReport(): Promise<LowStockRow[]> {
  await delay();
  return db.products
    .filter((p) => p.type === 'product' && p.active && p.stockQty <= (p.minStock ?? 0))
    .map((p) => ({
      productId: p.id,
      name: p.name,
      sku: p.sku,
      category: db.categories.find((c) => c.id === p.categoryId)?.name ?? 'بدون تصنيف',
      qty: p.stockQty,
      minStock: p.minStock ?? 0,
      suggestedQty: Math.max(p.reorderQty ?? (p.minStock ?? 0) * 2, (p.minStock ?? 0) - p.stockQty + (p.reorderQty ?? 0)),
      costValue: round2(p.stockQty * p.costPrice),
    }))
    .sort((a, b) => a.qty - b.qty);
});

export const getDeadStockReport = wrap('reports.getDeadStockReport', async function getDeadStockReport(days = 60): Promise<DeadStockRow[]> {
  await delay();
  const today = localDateKey(new Date());
  const lastSaleByProduct = new Map<string, string>();
  for (const m of db.stockMovements) {
    if (m.reason !== 'sale') continue;
    const prev = lastSaleByProduct.get(m.productId);
    if (!prev || m.date > prev) lastSaleByProduct.set(m.productId, m.date);
  }
  return db.products
    .filter((p) => p.type === 'product' && p.active && p.stockQty > 0)
    .map((p) => {
      const lastSaleDate = lastSaleByProduct.get(p.id);
      const daysSinceSale = lastSaleDate
        ? Math.floor((new Date(today).getTime() - new Date(localDateKey(lastSaleDate)).getTime()) / 86_400_000)
        : Number.MAX_SAFE_INTEGER;
      return {
        productId: p.id,
        name: p.name,
        sku: p.sku,
        category: db.categories.find((c) => c.id === p.categoryId)?.name ?? 'بدون تصنيف',
        qty: p.stockQty,
        costValue: round2(p.stockQty * p.costPrice),
        lastSaleDate,
        daysSinceSale,
      };
    })
    .filter((r) => r.daysSinceSale >= days)
    .sort((a, b) => b.costValue - a.costValue);
});

// --- Stocktake variances ---------------------------------------------------------------------

export const getStocktakeVariances = wrap('reports.getStocktakeVariances', async function getStocktakeVariances(countId?: string): Promise<StocktakeVarianceRow[]> {
  await delay();
  const counts = db.stockCounts.filter((c) => c.status === 'COMPLETED' && (!countId || c.id === countId));
  const rows: StocktakeVarianceRow[] = [];
  for (const c of counts) {
    for (const l of c.lines) {
      const product = db.products.find((p) => p.id === l.productId);
      if (!product) continue;
      const countedQty = l.countedQty ?? 0;
      const systemQty = l.systemQty ?? 0;
      const qtyVariance = round2(countedQty - systemQty);
      if (Math.abs(qtyVariance) < 0.0001) continue;
      rows.push({
        countId: c.id,
        countNumber: c.number,
        productId: l.productId,
        name: product.name,
        category: db.categories.find((cat) => cat.id === product.categoryId)?.name ?? 'بدون تصنيف',
        countedQty,
        systemQty,
        qtyVariance,
        valueVariance: round2(qtyVariance * product.costPrice),
      });
    }
  }
  return rows.sort((a, b) => Math.abs(b.valueVariance) - Math.abs(a.valueVariance));
});

// --- Transfers ------------------------------------------------------------------------------

export const getTransfersReport = wrap('reports.getTransfersReport', async function getTransfersReport(range: DateRangeInput): Promise<TransferReportRow[]> {
  await delay();
  return db.stockTransfers
    .filter((t) => inDateRange(t.date, range.from, range.to))
    .map((t) => {
      const sentQty = sum(t.lines, (l) => l.qty);
      const receivedQty = sum(t.lines, (l) => (l as { receivedQty?: number }).receivedQty ?? (t.status === 'RECEIVED' ? l.qty : 0));
      return {
        id: t.id,
        number: t.number,
        date: t.date,
        fromBranch: db.branches.find((b) => b.id === t.fromBranchId)?.name ?? t.fromBranchId,
        toBranch: db.branches.find((b) => b.id === t.toBranchId)?.name ?? t.toBranchId,
        status: t.status,
        sentQty,
        receivedQty,
        shortageQty: round2(sentQty - receivedQty),
        shortageValue: t.shortageValue ?? 0,
      };
    })
    .sort((a, b) => b.date.localeCompare(a.date));
});

// --- Purchases summary -----------------------------------------------------------------------

export const getPurchasesReport = wrap('reports.getPurchasesReport', async function getPurchasesReport(range: DateRangeInput): Promise<PurchasesReport> {
  await delay();
  const pos = db.purchaseOrders.filter((p) => p.status === 'RECEIVED' && inDateRange(p.date, range.from, range.to));
  const returns = db.purchaseReturns.filter((r) => inDateRange(r.date, range.from, range.to));

  const bySupplierMap = new Map<string, { count: number; total: number }>();
  for (const p of pos) {
    const row = bySupplierMap.get(p.supplierId) ?? { count: 0, total: 0 };
    row.count += 1;
    row.total = round2(row.total + p.grandTotal);
    bySupplierMap.set(p.supplierId, row);
  }
  const bySupplier = [...bySupplierMap.entries()]
    .map(([id, v]) => ({ supplierId: id, name: db.suppliers.find((s) => s.id === id)?.name ?? id, ...v }))
    .sort((a, b) => b.total - a.total);

  const byProductMap = new Map<string, { qty: number; total: number }>();
  for (const p of pos) {
    for (const l of p.lines) {
      const row = byProductMap.get(l.productId) ?? { qty: 0, total: 0 };
      row.qty += l.qty;
      row.total = round2(row.total + l.qty * l.costPrice);
      byProductMap.set(l.productId, row);
    }
  }
  const byProduct = [...byProductMap.entries()]
    .map(([id, v]) => ({ productId: id, name: db.products.find((p) => p.id === id)?.name ?? id, qty: round2(v.qty), total: v.total, avgPrice: v.qty > 0 ? round2(v.total / v.qty) : 0 }))
    .sort((a, b) => b.total - a.total);

  return {
    summary: {
      poCount: pos.length,
      grossPurchases: sum(pos, (p) => p.subTotal),
      vat: sum(pos, (p) => p.taxAmount),
      total: sum(pos, (p) => p.grandTotal),
      returns: sum(returns, (r) => r.grandTotal),
    },
    bySupplier,
    byProduct,
  };
});

// --- Expenses report -------------------------------------------------------------------------

export const getExpensesReport = wrap('reports.getExpensesReport', async function getExpensesReport(range: DateRangeInput): Promise<ExpensesReport> {
  await delay();
  const expenses = db.expenses.filter((e) => inDateRange(e.date, range.from, range.to));
  const byCategoryMap = new Map<string, number>();
  for (const e of expenses) byCategoryMap.set(e.categoryId, round2((byCategoryMap.get(e.categoryId) ?? 0) + e.amount));
  const byCategory = [...byCategoryMap.entries()]
    .map(([id, amount]) => ({ categoryId: id, name: db.expenseCategories.find((c) => c.id === id)?.name ?? id, amount }))
    .sort((a, b) => b.amount - a.amount);

  const byMonthMap = new Map<string, number>();
  for (const e of expenses) {
    const month = localDateKey(e.date).slice(0, 7);
    byMonthMap.set(month, round2((byMonthMap.get(month) ?? 0) + e.amount));
  }
  const byMonth = [...byMonthMap.entries()].map(([month, amount]) => ({ month, amount })).sort((a, b) => a.month.localeCompare(b.month));

  return { total: sum(expenses, (e) => e.amount), byCategory, byMonth };
});

// --- Period & branch comparison ----------------------------------------------------------------

/** v2 phase 12 (docs/v2/13 §2 "Period comparison: any two periods, key lines side by side with Δ and Δ%"). */
export const getPeriodComparison = wrap('reports.getPeriodComparison', async function getPeriodComparison(rangeA: DateRangeInput, rangeB: DateRangeInput): Promise<PeriodComparisonLine[]> {
  await delay();
  const a = computePnl(rangeA);
  const b = computePnl(rangeB);
  const delta = (x: number, y: number): PeriodComparisonLine => ({ label: '', a: x, b: y, delta: round2(x - y), deltaPct: y !== 0 ? round2(((x - y) / Math.abs(y)) * 100) : 0 });
  return [
    { ...delta(a.netRevenue, b.netRevenue), label: 'صافي الإيرادات' },
    { ...delta(a.totalCogs, b.totalCogs), label: 'تكلفة المبيعات' },
    { ...delta(a.grossProfit, b.grossProfit), label: 'مجمل الربح' },
    { ...delta(a.totalExpenses, b.totalExpenses), label: 'المصروفات' },
    { ...delta(a.netIncome, b.netIncome), label: 'صافي الربح' },
  ];
});

/** v2 phase 12 (docs/v2/13 §2 "Branch comparison: KPIs per branch"). */
export const getBranchComparison = wrap('reports.getBranchComparison', async function getBranchComparison(range: DateRangeInput): Promise<BranchComparisonRow[]> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  return db.branches
    .filter((b) => b.active)
    .map((b) => {
      const branchInvoices = invoices.filter((i) => i.branchId === b.id);
      const sales = sum(branchInvoices, (i) => i.grandTotal);
      const cogs = sum(branchInvoices, (i) =>
        sum(i.lines, (l) => (db.products.find((p) => p.id === l.productId)?.type === 'product' ? l.qty * l.costPrice : 0)),
      );
      return {
        branchId: b.id,
        name: b.name,
        sales: round2(sales),
        grossProfit: round2(sum(branchInvoices, (i) => i.subTotal - i.discountAmount) - cogs),
        invoiceCount: branchInvoices.length,
        averageInvoice: branchInvoices.length ? round2(sales / branchInvoices.length) : 0,
      };
    })
    .sort((a, b) => b.sales - a.sales);
});

// --- Business health ------------------------------------------------------------------------

/**
 * v2 phase 12 (docs/v2/13 §2 "Business health"): liquidity/profitability/debt/collection each
 * scored 0–25 from the reference thresholds. Plain, explainable scoring — not a machine-learned
 * model — matching the rest of this codebase's insight rules.
 */
export const getBusinessHealthReport = wrap('reports.getBusinessHealthReport', async function getBusinessHealthReport(range: DateRangeInput): Promise<BusinessHealthReport> {
  await delay();
  const asOf = range.to ?? localDateKey(new Date());
  const bs = await getBalanceSheet(asOf);
  const pnl = computePnl(range);

  const currentAssets = sum(bs.assets.filter((l) => db.accounts.find((a) => a.id === l.accountId)?.subtype !== 'fixedAsset'), (l) => l.amount);
  const currentLiabilities = sum(bs.liabilities, (l) => l.amount); // no long/short split modeled yet — treated as current
  const currentRatio = currentLiabilities > 0 ? currentAssets / currentLiabilities : currentAssets > 0 ? 2 : 1;
  const liquidityScore = Math.max(0, Math.min(25, round2((currentRatio / 2) * 25)));

  const netMarginPct = pnl.netRevenue > 0 ? (pnl.netIncome / pnl.netRevenue) * 100 : 0;
  const profitabilityScore = Math.max(0, Math.min(25, round2((netMarginPct / 20) * 25)));

  const totalDebtToEquity = bs.totalEquity > 0 ? bs.totalLiabilities / bs.totalEquity : bs.totalLiabilities > 0 ? 2 : 0;
  const debtScore = Math.max(0, Math.min(25, round2(25 - (totalDebtToEquity / 1) * 25)));

  // Collection: DSO = (AR balance / net credit sales) × days in period.
  const arAccount = db.accounts.find((a) => !a.isGroup && a.subtype === 'receivable');
  const arBalance = arAccount ? sum(bs.assets.filter((l) => l.accountId === arAccount.id), (l) => l.amount) : 0;
  const days = range.from && range.to ? Math.max(1, Math.round((new Date(range.to).getTime() - new Date(range.from).getTime()) / 86_400_000)) : 30;
  const dso = pnl.netRevenue > 0 ? (arBalance / pnl.netRevenue) * days : 0;
  const collectionScore = Math.max(0, Math.min(25, round2(25 - (dso / 60) * 25)));

  const scores: BusinessHealthReport['scores'] = [
    { key: 'liquidity', label: 'السيولة', score: liquidityScore, value: round2(currentRatio), explanation: `نسبة التداول ${round2(currentRatio)} — الأصول المتداولة مقابل الالتزامات المتداولة` },
    { key: 'profitability', label: 'الربحية', score: profitabilityScore, value: round2(netMarginPct), explanation: `هامش صافي الربح ${round2(netMarginPct)}%` },
    { key: 'debt', label: 'المديونية', score: debtScore, value: round2(totalDebtToEquity), explanation: `نسبة الالتزامات إلى حقوق الملكية ${round2(totalDebtToEquity)}` },
    { key: 'collection', label: 'التحصيل', score: collectionScore, value: round2(dso), explanation: `متوسط أيام تحصيل الذمم (DSO) ${round2(dso)} يوماً` },
  ];
  return { total: round2(sum(scores, (s) => s.score)), scores };
});

// --- Profit leakage --------------------------------------------------------------------------

/** v2 phase 12 (docs/v2/13 §2 "Profit leakage: discounts + price overrides + returns + shrinkage + write-offs as % of sales"). */
export const getProfitLeakageReport = wrap('reports.getProfitLeakageReport', async function getProfitLeakageReport(range: DateRangeInput): Promise<ProfitLeakageReport> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));
  const netSales = round2(sum(invoices, (i) => i.subTotal - i.discountAmount));
  const discounts = round2(sum(invoices, (i) => i.discountAmount) + sum(invoices, (i) => sum(i.lines, (l) => l.discount)));
  const returns = sum(refunds, (r) => r.subTotal);
  // Shrinkage/write-offs: LOSS-type stock adjustments' journal value in the period.
  const writeOffs = sum(
    db.stockMovements.filter((m) => m.reason === 'loss' && inDateRange(m.date, range.from, range.to)),
    (m) => Math.abs(m.valueChange),
  );
  const shrinkage = sum(
    db.stockMovements.filter((m) => m.reason === 'stocktake' && m.qtyChange < 0 && inDateRange(m.date, range.from, range.to)),
    (m) => Math.abs(m.valueChange),
  );
  const totalLeakage = round2(discounts + returns + writeOffs + shrinkage);
  return { netSales, discounts, returns, writeOffs, shrinkage, totalLeakage, leakagePct: netSales > 0 ? round2((totalLeakage / netSales) * 100) : 0 };
});

// --- Filter lookups for dimension selects ------------------------------------------------------

/** v2 phase 12 (docs/v2/10 §4): filter options for `ReportShell`'s branch/cost-center/currency selects — empty unless the matching feature switch is on. */
export const getDimensionOptions = wrap('reports.getDimensionOptions', async function getDimensionOptions(): Promise<{
  branches: { id: string; label: string }[];
  costCenters: { id: string; label: string }[];
  currencies: { code: string; label: string }[];
}> {
  await delay(60);
  return {
    branches: db.branches.filter((b) => b.active).map((b) => ({ id: b.id, label: b.name })),
    costCenters: db.costCenters.filter((c) => c.active).map((c) => ({ id: c.id, label: c.name })),
    currencies: db.currencies.filter((c) => c.active).map((c) => ({ code: c.code, label: `${c.nameAr} (${c.code})` })),
  };
});

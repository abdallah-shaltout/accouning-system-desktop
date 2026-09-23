import { ApiError, db, delay, inDateRange, localDateKey, round2, sum } from '@/mocks';
import { accountFor } from '@/mocks/backend/accounts';
import { customerStatement, supplierStatement } from '@/mocks/backend/balances';
import type { Account, AccountKind } from '@/modules/accounting/types';
import { SALE_METHOD_LABEL } from '@/modules/core/helpers/labels';
import type {
  AccountLedger,
  BalanceSheet,
  DateRangeInput,
  InventoryReportRow,
  ProfitAndLoss,
  SalesReport,
  StatementLine,
  TrialBalanceRow,
  VatCategoryBox,
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

/** Σ(debit − credit) per account for entries within the range. */
function movements(range: DateRangeInput): Map<string, { d: number; c: number }> {
  const map = new Map<string, { d: number; c: number }>();
  for (const e of db.journalEntries) {
    if (!inDateRange(e.date, range.from, range.to)) continue;
    for (const l of e.lines) {
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

export async function getTrialBalance(range: DateRangeInput): Promise<TrialBalanceRow[]> {
  await delay();
  const opening = range.from ? movements({ to: dayBefore(range.from) }) : new Map();
  const period = movements(range);
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
}

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

function computePnl(range: DateRangeInput): ProfitAndLoss {
  const mv = movements(range);
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

export async function getProfitAndLoss(range: DateRangeInput): Promise<ProfitAndLoss> {
  await delay();
  return computePnl(range);
}

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

export async function getCostCenterProfitAndLoss(range: DateRangeInput): Promise<import('../types').CostCenterPnl> {
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
}

/** Budget vs actual (docs/v2/10 §3): actual = Σ expense-account movement for the cost center within the fiscal year's dates. */
export async function getCostCenterBudgetVsActual(fiscalYearId: string): Promise<import('../types').CostCenterBudgetRow[]> {
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
    // TODO(phase 10): surface this as a real insight (insight engine, dismiss/snooze) — this phase
    // only computes the 90%-of-budget signal as a plain boolean column on the report.
    rows.push({ costCenterId: c.id, name: c.name, budget: budgetRow.amount, actual, variancePct, nearBudget: budgetRow.amount > 0 && actual >= budgetRow.amount * 0.9 });
  }
  return rows.sort((a, b) => b.actual - a.actual);
}

// --- Balance sheet ---------------------------------------------------------------------------

export async function getBalanceSheet(asOf: string): Promise<BalanceSheet> {
  await delay();
  const mv = movements({ to: asOf });
  const assets = lines('ASSET', mv, 1);
  const liabilities = lines('LIABILITY', mv, -1);
  const equity = lines('EQUITY', mv, -1);
  const unclosedEarnings = computePnl({ to: asOf }).netIncome;
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
}

// --- Ledgers / statements ---------------------------------------------------------------------

export async function getAccountLedger(accountId: string, range: DateRangeInput): Promise<AccountLedger> {
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
}

/** Customer / supplier statement in the same shape as an account ledger. */
export async function getPartyLedger(kind: 'customer' | 'supplier', partyId: string, range: DateRangeInput): Promise<AccountLedger> {
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
}

// --- Sales -----------------------------------------------------------------------------------

export async function getSalesReport(range: DateRangeInput): Promise<SalesReport> {
  await delay();
  const invoices = db.invoices.filter((i) => i.status !== 'DRAFT' && inDateRange(i.date, range.from, range.to));
  const refunds = db.refunds.filter((r) => inDateRange(r.date, range.from, range.to));

  const byProduct = new Map<string, SalesReport['byProduct'][number]>();
  for (const inv of invoices) {
    const factor = 1 - inv.discountRate / 100;
    for (const l of inv.lines) {
      const row = byProduct.get(l.productId) ?? { productId: l.productId, name: l.name, qty: 0, revenue: 0, cost: 0, profit: 0 };
      const product = db.products.find((p) => p.id === l.productId);
      row.qty += l.qty;
      row.revenue += (l.qty * l.price - l.discount) * factor;
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
  const netSales = round2(grossSales - discounts);
  const vat = sum(invoices, (i) => i.taxAmount);
  const total = sum(invoices, (i) => i.grandTotal);
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
}

// --- Inventory -------------------------------------------------------------------------------

export async function getInventoryReport(): Promise<InventoryReportRow[]> {
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
}

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

export async function getVatReport(range: DateRangeInput): Promise<VatReport> {
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
}

// --- Lookups for report filters ---------------------------------------------------------------

export async function getLedgerTargets(): Promise<{ accounts: { id: string; label: string }[]; customers: { id: string; label: string }[]; suppliers: { id: string; label: string }[] }> {
  await delay(80);
  return {
    accounts: [...db.accounts].sort((a, b) => a.code.localeCompare(b.code)).map((a) => ({ id: a.id, label: `${a.code} — ${a.name}` })),
    customers: db.customers.map((c) => ({ id: c.id, label: c.name })),
    suppliers: db.suppliers.map((s) => ({ id: s.id, label: s.name })),
  };
}

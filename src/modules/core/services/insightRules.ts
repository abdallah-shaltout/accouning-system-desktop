import {
  AlertTriangle,
  Banknote,
  CalendarRange,
  Clock,
  CreditCard,
  DatabaseBackup,
  FileWarning,
  Landmark,
  PackageX,
  Percent,
  PiggyBank,
  Repeat,
  RotateCcw,
  ScrollText,
  Sparkles,
  TimerOff,
  TrendingDown,
  UserX,
} from '@lucide/vue';
import { db } from '@/mocks/db';
import { localDateKey, round2, sum } from '@/mocks/utils';
import { accountFor } from '@/mocks/backend/accounts';
import { customerBalance } from '@/mocks/backend/balances';
import { getOpenDocumentsFor } from '@/mocks/backend/payments';
import { formatMoney, formatNumber } from '@/modules/core/helpers/format';
import { invoiceOutstanding } from '@/modules/invoices/helpers/totals';
import { isOverdue } from '@/modules/invoices/services/invoiceService';
import { useBackupStore } from '@/modules/settings/controllers/useBackupStore';
import type { InsightRule, InsightSeverity } from './insightTypes';

/**
 * Rule catalogue (docs/v2/11-journal-dashboard-insights.md D2, first set of 20). Each rule is a
 * pure function `(ctx) => Insight[]` reading straight from `db` (mirrors
 * `src/mocks/backend/*` conventions: no `await`/service call needed since the engine already runs
 * inside the same synchronous mock-data world). All ids are `${ruleKey}:${entityId}` so
 * dismiss/snooze targets one instance, not the whole rule.
 */

function daysBetween(a: string, b: string): number {
  return Math.floor((new Date(a).getTime() - new Date(b).getTime()) / 86_400_000);
}

function accountBalance(role: Parameters<typeof accountFor>[0]): number {
  const id = accountFor(role).id;
  let total = 0;
  for (const entry of db.journalEntries) for (const line of entry.lines) if (line.accountId === id) total += line.debit - line.credit;
  return round2(total);
}

// 1 — Reorder (storekeeper, manager) ------------------------------------------------------------
const reorderRule: InsightRule = (ctx) => {
  const low = db.products.filter((p) => p.active && p.type === 'product' && p.stockMode !== 'none' && p.stockQty <= (p.minStock ?? 0));
  if (!low.length) return [];
  const bySupplier = new Map<string, typeof low>();
  for (const p of low) {
    const key = p.preferredSupplierId ?? '__none__';
    if (!bySupplier.has(key)) bySupplier.set(key, []);
    bySupplier.get(key)!.push(p);
  }
  const insights = [];
  for (const [supplierId, items] of bySupplier) {
    const supplier = db.suppliers.find((s) => s.id === supplierId);
    const label = supplier ? `لدى مورد "${supplier.name}"` : 'بلا مورد مفضل';
    insights.push({
      id: `reorder:${supplierId}`,
      ruleKey: 'reorder',
      severity: 'warning' as const,
      message: `${items.length} ${items.length === 1 ? 'صنف' : 'أصناف'} عند حد الطلب ${label}`,
      metric: `${formatNumber(items.length)} صنف`,
      actionLabel: 'إنشاء أمر شراء',
      actionTo: supplier ? { path: '/purchases/new', query: { supplier: supplierId } } : { path: '/products', query: { stock: 'low' } },
      icon: PackageX,
      roles: ['storekeeper', 'manager', 'admin'] as const,
      value: items.length * 100,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 2 — Dead stock (manager) -----------------------------------------------------------------------
const deadStockRule: InsightRule = (ctx) => {
  const cutoff = new Date(ctx.today);
  cutoff.setDate(cutoff.getDate() - ctx.thresholds.deadStockDays);
  const cutoffKey = localDateKey(cutoff);
  const lastSaleByProduct = new Map<string, string>();
  for (const inv of db.invoices) {
    if (inv.status === 'DRAFT') continue;
    for (const line of inv.lines) {
      const prev = lastSaleByProduct.get(line.productId);
      if (!prev || inv.date > prev) lastSaleByProduct.set(line.productId, inv.date);
    }
  }
  const stale = db.products.filter((p) => {
    if (!p.active || p.type !== 'product' || p.stockMode === 'none' || p.stockQty <= 0) return false;
    if ((p.stockValue ?? 0) < ctx.thresholds.deadStockValue) return false;
    const lastSale = lastSaleByProduct.get(p.id);
    return !lastSale || localDateKey(lastSale) < cutoffKey;
  });
  if (!stale.length) return [];
  const value = round2(sum(stale, (p) => p.stockValue ?? 0));
  return [
    {
      id: 'dead-stock:all',
      ruleKey: 'dead-stock',
      severity: 'warning',
      message: `${stale.length} صنف بلا مبيعات منذ ${ctx.thresholds.deadStockDays} يوماً — بضاعة راكدة بقيمة ${formatMoney(value)}`,
      metric: formatMoney(value),
      actionLabel: 'عرض القائمة',
      actionTo: '/products',
      icon: TrendingDown,
      roles: ['manager', 'admin'],
      value,
      createdAt: ctx.today,
    },
  ];
};

// 3 — Expiring batches (storekeeper, manager) ----------------------------------------------------
const expiringRule: InsightRule = (ctx) => {
  const soon = db.productBatches.filter((b) => {
    if (b.qty <= 0.0001 || !b.expiryDate) return false;
    const days = daysBetween(b.expiryDate, ctx.today);
    return days >= 0 && days <= ctx.thresholds.expiryAlertDays;
  });
  if (!soon.length) return [];
  return [
    {
      id: 'expiring:all',
      ruleKey: 'expiring',
      severity: 'warning',
      message: `${soon.length} ${soon.length === 1 ? 'تشغيلة تنتهي' : 'تشغيلات تنتهي'} خلال ${ctx.thresholds.expiryAlertDays} يوماً`,
      metric: `${formatNumber(soon.length)} تشغيلة`,
      actionLabel: 'تقرير الصلاحية',
      actionTo: '/inventory/expiry',
      icon: TimerOff,
      roles: ['storekeeper', 'manager', 'admin'],
      value: soon.length * 80,
      createdAt: ctx.today,
    },
  ];
};

// 4 — Overdue customers (accountant, manager) ----------------------------------------------------
const overdueCustomersRule: InsightRule = (ctx) => {
  const overdueInvoices = db.invoices.filter((i) => isOverdue(i));
  if (!overdueInvoices.length) return [];
  const byCustomer = new Map<string, { total: number; maxDays: number }>();
  for (const inv of overdueInvoices) {
    if (!inv.customerId) continue;
    const days = Math.max(0, daysBetween(ctx.today, localDateKey(inv.dueDate!)));
    const entry = byCustomer.get(inv.customerId) ?? { total: 0, maxDays: 0 };
    entry.total = round2(entry.total + invoiceOutstanding(inv));
    entry.maxDays = Math.max(entry.maxDays, days);
    byCustomer.set(inv.customerId, entry);
  }
  const insights = [];
  for (const [customerId, { total, maxDays }] of byCustomer) {
    const customer = db.customers.find((c) => c.id === customerId);
    if (!customer || total <= 0) continue;
    insights.push({
      id: `overdue-customer:${customerId}`,
      ruleKey: 'overdue-customers',
      severity: 'critical' as const,
      message: `"${customer.name}" متأخر ${formatNumber(maxDays)} يوماً بمبلغ ${formatMoney(total)}`,
      metric: formatMoney(total),
      actionLabel: 'كشف الحساب',
      actionTo: `/customers/${customerId}`,
      icon: UserX,
      roles: ['accountant', 'manager', 'admin'] as const,
      value: total,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 5 — Credit limit near/over (cashier, accountant, manager) --------------------------------------
const creditLimitRule: InsightRule = (ctx) => {
  const insights = [];
  for (const c of db.customers) {
    if (!c.creditLimit || c.creditLimit <= 0) continue;
    const balance = customerBalance(c.id);
    if (balance < c.creditLimit * 0.9) continue;
    const over = balance > c.creditLimit;
    insights.push({
      id: `credit-limit:${c.id}`,
      ruleKey: 'credit-limit',
      severity: (over ? 'critical' : 'warning') as InsightSeverity,
      message: over
        ? `"${c.name}" تجاوز الحد الائتماني: الرصيد ${formatMoney(balance)} من أصل ${formatMoney(c.creditLimit)}`
        : `"${c.name}" قريب من الحد الائتماني: ${formatMoney(balance)} من أصل ${formatMoney(c.creditLimit)}`,
      metric: formatMoney(balance),
      actionLabel: 'عرض العميل',
      actionTo: `/customers/${c.id}`,
      icon: CreditCard,
      roles: ['cashier', 'accountant', 'manager', 'admin'] as const,
      value: balance,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 6 — Supplier dues vs cash (accountant, manager) ------------------------------------------------
const supplierDuesRule: InsightRule = (ctx) => {
  const cutoff = new Date(ctx.today);
  cutoff.setDate(cutoff.getDate() + ctx.thresholds.supplierDueDays);
  const cutoffKey = localDateKey(cutoff);
  let dueThisWeek = 0;
  for (const s of db.suppliers) {
    const docs = getOpenDocumentsFor('supplier', s.id);
    for (const d of docs) {
      const refDate = localDateKey(d.dueDate ?? d.date);
      if (refDate <= cutoffKey) dueThisWeek = round2(dueThisWeek + d.outstanding);
    }
  }
  if (dueThisWeek <= 0) return [];
  const cash = round2(accountBalance('cash') + accountBalance('bank'));
  if (dueThisWeek < cash) return [];
  return [
    {
      id: 'supplier-dues:all',
      ruleKey: 'supplier-dues',
      severity: 'warning',
      message: `مستحقات ${formatMoney(dueThisWeek)} خلال ${ctx.thresholds.supplierDueDays} أيام — السيولة المتاحة ${formatMoney(cash)}`,
      metric: formatMoney(dueThisWeek),
      actionLabel: 'التخطيط للسداد',
      actionTo: '/payments',
      icon: Landmark,
      roles: ['accountant', 'manager', 'admin'],
      value: dueThisWeek,
      createdAt: ctx.today,
    },
  ];
};

// 7 — VAT deadline (accountant, manager) -----------------------------------------------------------
const vatDeadlineRule: InsightRule = (ctx) => {
  const today = new Date(ctx.today);
  // Quarterly period ending on the last day of the month before this one, deadline = end of the following month.
  const periodEnd = new Date(today.getFullYear(), today.getMonth(), 0);
  const deadline = new Date(periodEnd.getFullYear(), periodEnd.getMonth() + 2, 0);
  const daysLeft = daysBetween(localDateKey(deadline), ctx.today) * -1;
  if (daysLeft < 0 || daysLeft > ctx.thresholds.vatDeadlineDays) return [];
  const quarter = Math.floor(periodEnd.getMonth() / 3) + 1;
  return [
    {
      id: `vat-deadline:${periodEnd.getFullYear()}-q${quarter}`,
      ruleKey: 'vat-deadline',
      severity: 'warning',
      message: `إقرار الربع ${quarter} مستحق خلال ${formatNumber(daysLeft)} ${daysLeft === 1 ? 'يوم' : 'أيام'}`,
      metric: `${formatNumber(daysLeft)} يوم`,
      actionLabel: 'تسوية الضريبة',
      actionTo: '/accounting/vat-settlement',
      icon: FileWarning,
      roles: ['accountant', 'manager', 'admin'],
      value: (ctx.thresholds.vatDeadlineDays - daysLeft + 1) * 1000,
      createdAt: ctx.today,
    },
  ];
};

// 8 — Cash in drawer (manager, cashier) -----------------------------------------------------------
const cashInDrawerRule: InsightRule = (ctx) => {
  const cashOnHand = accountBalance('cash');
  if (cashOnHand <= ctx.thresholds.cashDrawerLimit) return [];
  return [
    {
      id: 'cash-drawer:all',
      ruleKey: 'cash-drawer',
      severity: 'warning',
      message: `النقدية في الصندوق ${formatMoney(cashOnHand)} — أودِعها في البنك`,
      metric: formatMoney(cashOnHand),
      actionLabel: 'سند تحويل',
      actionTo: '/vouchers',
      icon: Banknote,
      roles: ['manager', 'admin', 'cashier'],
      value: cashOnHand,
      createdAt: ctx.today,
    },
  ];
};

// 9 — Shift not closed (manager) -------------------------------------------------------------------
const shiftNotClosedRule: InsightRule = (ctx) => {
  const now = new Date(ctx.today).getTime();
  const stuck = db.shifts.filter((s) => {
    if (s.status !== 'OPEN') return false;
    const hours = (now - new Date(s.openedAt).getTime()) / 3_600_000;
    return hours >= ctx.thresholds.shiftOpenHours;
  });
  return stuck.map((s) => {
    const hours = Math.floor((now - new Date(s.openedAt).getTime()) / 3_600_000);
    return {
      id: `shift-open:${s.id}`,
      ruleKey: 'shift-open',
      severity: 'warning' as const,
      message: `وردية ${s.number} مفتوحة منذ ${formatNumber(hours)} ساعة`,
      metric: `${formatNumber(hours)} ساعة`,
      actionLabel: 'إغلاق قسري',
      actionTo: '/pos/shifts',
      icon: Clock,
      roles: ['manager', 'admin'] as const,
      value: hours,
      createdAt: ctx.today,
    };
  });
};

// 10 — Unsettled card/wallet clearing (accountant) -------------------------------------------------
const unsettledCardsRule: InsightRule = (ctx) => {
  const cutoff = new Date(ctx.today);
  cutoff.setDate(cutoff.getDate() - ctx.thresholds.unsettledClearingDays);
  const cutoffKey = localDateKey(cutoff);
  const unsettledLines = db.journalEntries
    .flatMap((e) => e.lines.map((l) => ({ ...l, entry: e })))
    .filter((l) => {
      const role = db.accounts.find((a) => a.id === l.accountId)?.systemRole;
      return (role === 'cardClearing' || role === 'walletClearing') && l.debit > 0 && localDateKey(e_date(l)) <= cutoffKey;
    });
  function e_date(l: { entry: { date: string } }) {
    return l.entry.date;
  }
  if (!unsettledLines.length) return [];
  const total = round2(sum(unsettledLines, (l) => l.debit));
  return [
    {
      id: 'unsettled-cards:all',
      ruleKey: 'unsettled-cards',
      severity: 'info',
      message: `تحصيلات بطاقات/محافظ غير مسواة منذ أكثر من ${ctx.thresholds.unsettledClearingDays} أيام بقيمة ${formatMoney(total)}`,
      metric: formatMoney(total),
      actionLabel: 'تسوية البطاقات',
      actionTo: '/payments/settlements',
      icon: CreditCard,
      roles: ['accountant', 'admin'],
      value: total,
      createdAt: ctx.today,
    },
  ];
};

// 11 — Below-cost prices (manager) -------------------------------------------------------------------
const belowCostRule: InsightRule = (ctx) => {
  const bad = db.products.filter((p) => {
    if (!p.active || p.type !== 'product' || p.price <= 0) return false;
    if (p.price <= p.costPrice) return true;
    const margin = ((p.price - p.costPrice) / p.price) * 100;
    return margin < ctx.thresholds.minMarginPct;
  });
  if (!bad.length) return [];
  return [
    {
      id: 'below-cost:all',
      ruleKey: 'below-cost',
      severity: 'warning',
      message: `${bad.length} صنف بسعر أقل من التكلفة أو بهامش ربح ضعيف`,
      metric: `${formatNumber(bad.length)} صنف`,
      actionLabel: 'مراجعة الأسعار',
      actionTo: '/products',
      icon: TrendingDown,
      roles: ['manager', 'admin'],
      value: bad.length * 50,
      createdAt: ctx.today,
    },
  ];
};

// 12 — Discount leak (manager) -----------------------------------------------------------------------
const discountLeakRule: InsightRule = (ctx) => {
  const weekAgo = new Date(ctx.today);
  weekAgo.setDate(weekAgo.getDate() - 7);
  const weekAgoKey = localDateKey(weekAgo);
  const thisWeek = db.invoices.filter((i) => i.status !== 'DRAFT' && localDateKey(i.date) >= weekAgoKey && i.grandTotal > 0);
  if (thisWeek.length < 4) return [];
  const byCashier = new Map<string, number[]>();
  for (const inv of thisWeek) {
    if (!inv.cashierId) continue;
    const rate = inv.discountRate ?? 0;
    if (!byCashier.has(inv.cashierId)) byCashier.set(inv.cashierId, []);
    byCashier.get(inv.cashierId)!.push(rate);
  }
  const allRates = thisWeek.map((i) => i.discountRate ?? 0);
  const avg = allRates.length ? sum(allRates.map((r) => ({ r })), (x) => x.r) / allRates.length : 0;
  if (avg <= 0) return [];
  const insights = [];
  for (const [cashierId, rates] of byCashier) {
    if (rates.length < 3) continue;
    const cashierAvg = sum(rates.map((r) => ({ r })), (x) => x.r) / rates.length;
    if (cashierAvg < avg * ctx.thresholds.discountLeakMultiplier) continue;
    const user = db.users.find((u) => u.id === cashierId);
    insights.push({
      id: `discount-leak:${cashierId}`,
      ruleKey: 'discount-leak',
      severity: 'warning' as const,
      message: `متوسط خصم "${user?.name ?? cashierId}" ${formatNumber(cashierAvg, 1)}% مقابل ${formatNumber(avg, 1)}% لبقية الكاشيرين هذا الأسبوع`,
      metric: `${formatNumber(cashierAvg, 1)}%`,
      actionLabel: 'تقرير المبيعات',
      actionTo: '/reports/sales',
      icon: Percent,
      roles: ['manager', 'admin'] as const,
      value: cashierAvg,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 13 — Refund spike (manager) ------------------------------------------------------------------------
const refundSpikeRule: InsightRule = (ctx) => {
  const weekAgo = new Date(ctx.today);
  weekAgo.setDate(weekAgo.getDate() - 7);
  const weekAgoKey = localDateKey(weekAgo);
  const fourWeeksAgo = new Date(ctx.today);
  fourWeeksAgo.setDate(fourWeeksAgo.getDate() - 28);
  const fourWeeksAgoKey = localDateKey(fourWeeksAgo);

  const thisWeek = db.refunds.filter((r) => localDateKey(r.date) >= weekAgoKey);
  const priorWeeks = db.refunds.filter((r) => localDateKey(r.date) >= fourWeeksAgoKey && localDateKey(r.date) < weekAgoKey);
  if (!thisWeek.length) return [];
  const thisWeekTotal = sum(thisWeek, (r) => r.grandTotal);
  const priorAvgWeekly = priorWeeks.length ? sum(priorWeeks, (r) => r.grandTotal) / 3 : 0;
  if (priorAvgWeekly <= 0 || thisWeekTotal < priorAvgWeekly * ctx.thresholds.refundSpikeMultiplier) return [];
  return [
    {
      id: 'refund-spike:all',
      ruleKey: 'refund-spike',
      severity: 'warning',
      message: `المرتجعات هذا الأسبوع ${formatMoney(thisWeekTotal)} — أعلى من المتوسط (${formatMoney(round2(priorAvgWeekly))})`,
      metric: formatMoney(thisWeekTotal),
      actionLabel: 'تقرير المبيعات',
      actionTo: '/reports/sales',
      icon: RotateCcw,
      roles: ['manager', 'admin'],
      value: thisWeekTotal,
      createdAt: ctx.today,
    },
  ];
};

// 14/14b — Recurring due: journal + expenses (accountant) -----------------------------------------
const recurringJournalDueRule: InsightRule = (ctx) => {
  const due = db.journalTemplates.filter((t) => t.recurrence && t.recurrence.nextDate <= ctx.today);
  return due.map((t) => ({
    id: `recurring-journal:${t.id}`,
    ruleKey: 'recurring-journal-due',
    severity: 'info' as const,
    message: `قيد متكرر "${t.name}" مستحق الترحيل`,
    actionLabel: 'ترحيل الآن',
    actionTo: '/accounting/journal-templates',
    icon: Repeat,
    roles: ['accountant', 'admin'] as const,
    value: 200,
    createdAt: ctx.today,
  }));
};

const recurringExpenseDueRule: InsightRule = (ctx) => {
  const due = db.recurringExpenses.filter((r) => r.active && r.nextDate <= ctx.today);
  return due.map((r) => ({
    id: `recurring-expense:${r.id}`,
    ruleKey: 'recurring-expense-due',
    severity: 'info' as const,
    message: `مصروف متكرر "${r.name}" بمبلغ ${formatMoney(r.amount)} مستحق التسجيل`,
    metric: formatMoney(r.amount),
    actionLabel: 'تسجيل الآن',
    actionTo: '/expenses',
    icon: Repeat,
    roles: ['accountant', 'admin'] as const,
    value: r.amount,
    createdAt: ctx.today,
  }));
};

// 15 — Budget (manager) ---------------------------------------------------------------------------
const budgetRule: InsightRule = (ctx) => {
  const fy = db.fiscalYears.find((f) => !f.isClosed && f.startDate <= ctx.today && f.endDate >= ctx.today) ?? db.fiscalYears.find((f) => !f.isClosed);
  if (!fy) return [];
  const insights = [];
  for (const c of db.costCenters) {
    const budgetRow = c.budgets?.find((b) => b.fiscalYearId === fy.id);
    if (!budgetRow || budgetRow.amount <= 0) continue;
    const actual = sum(
      db.journalEntries.flatMap((e) => e.lines.filter((l) => l.costCenterId === c.id && e.date >= fy.startDate && e.date <= fy.endDate)),
      (l) => {
        const account = db.accounts.find((a) => a.id === l.accountId);
        return account?.kind === 'EXPENSE' ? l.debit - l.credit : 0;
      },
    );
    const pct = (actual / budgetRow.amount) * 100;
    if (pct < ctx.thresholds.budgetNearPct) continue;
    insights.push({
      id: `budget:${c.id}`,
      ruleKey: 'budget',
      severity: (pct >= 100 ? 'critical' : 'warning') as InsightSeverity,
      message: `مركز التكلفة "${c.name}" تجاوز ${formatNumber(pct, 0)}% من ميزانيته (${formatMoney(actual)} من ${formatMoney(budgetRow.amount)})`,
      metric: `${formatNumber(pct, 0)}%`,
      actionLabel: 'تقرير مراكز التكلفة',
      actionTo: '/reports/cost-centers',
      icon: PiggyBank,
      roles: ['manager', 'admin'] as const,
      value: actual,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 16 — Opening balance equity (accountant) ----------------------------------------------------------
const openingBalanceEquityRule: InsightRule = (ctx) => {
  const balance = accountBalance('openingBalanceEquity');
  if (Math.abs(balance) < 0.01) return [];
  return [
    {
      id: 'opening-balance-equity:all',
      ruleKey: 'opening-balance-equity',
      severity: 'info',
      message: `رصيد حساب "أرصدة افتتاحية" ${formatMoney(balance)} — يحتاج إقفال إلى رأس المال`,
      metric: formatMoney(balance),
      actionLabel: 'قيد يومية',
      actionTo: '/accounting/journal/new',
      icon: ScrollText,
      roles: ['accountant', 'admin'],
      value: Math.abs(balance),
      createdAt: ctx.today,
    },
  ];
};

// 17 — Backup overdue (admin) -----------------------------------------------------------------------
const backupOverdueRule: InsightRule = (ctx) => {
  const backupStore = useBackupStore();
  const lastBackupAt = backupStore.lastBackupAt;
  const daysSince = lastBackupAt ? daysBetween(ctx.today, localDateKey(lastBackupAt)) : Infinity;
  if (daysSince < ctx.thresholds.backupOverdueDays) return [];
  return [
    {
      id: 'backup-overdue:all',
      ruleKey: 'backup-overdue',
      severity: 'critical',
      message: lastBackupAt ? `لم يتم أخذ نسخة احتياطية منذ ${formatNumber(daysSince)} يوماً` : 'لم يتم أخذ أي نسخة احتياطية بعد',
      metric: Number.isFinite(daysSince) ? `${formatNumber(daysSince)} يوم` : undefined,
      actionLabel: 'أخذ نسخة الآن',
      actionTo: '/settings/backup',
      icon: DatabaseBackup,
      roles: ['admin'],
      value: Number.isFinite(daysSince) ? daysSince : 9999,
      createdAt: ctx.today,
    },
  ];
};

// 18 — Year end (accountant) -----------------------------------------------------------------------
const yearEndRule: InsightRule = (ctx) => {
  const insights = [];
  for (const fy of db.fiscalYears) {
    if (fy.isClosed) continue;
    const daysToEnd = daysBetween(fy.endDate, ctx.today) * -1;
    if (daysToEnd < -365 || daysToEnd > ctx.thresholds.yearEndDays) continue;
    const ended = daysToEnd < 0;
    insights.push({
      id: `year-end:${fy.id}`,
      ruleKey: 'year-end',
      severity: (ended ? 'critical' : 'warning') as InsightSeverity,
      message: ended ? `السنة المالية "${fy.name}" انتهت ولم تُقفل بعد` : `السنة المالية "${fy.name}" تنتهي خلال ${formatNumber(daysToEnd)} يوماً`,
      metric: ended ? undefined : `${formatNumber(daysToEnd)} يوم`,
      actionLabel: 'إغلاق السنة',
      actionTo: '/accounting/fiscal-years',
      icon: CalendarRange,
      roles: ['accountant', 'admin'] as const,
      value: ended ? 5000 : ctx.thresholds.yearEndDays - daysToEnd,
      createdAt: ctx.today,
    });
  }
  return insights;
};

// 19 — Good news: best sales day/week (manager, positive, no action) --------------------------------
const goodNewsRule: InsightRule = (ctx) => {
  const byDay = new Map<string, number>();
  for (const inv of db.invoices) {
    if (inv.status === 'DRAFT') continue;
    const key = localDateKey(inv.date);
    byDay.set(key, round2((byDay.get(key) ?? 0) + inv.grandTotal));
  }
  if (byDay.get(ctx.today) === undefined) return [];
  const todayTotal = byDay.get(ctx.today) ?? 0;
  if (todayTotal <= 0) return [];
  const others = [...byDay.entries()].filter(([d]) => d !== ctx.today).map(([, v]) => v);
  const best = Math.max(0, ...others);
  if (todayTotal <= best) return [];
  return [
    {
      id: `good-news:${ctx.today}`,
      ruleKey: 'good-news',
      severity: 'positive',
      message: `اليوم أفضل يوم مبيعات مسجل بقيمة ${formatMoney(todayTotal)} 🎉`,
      metric: formatMoney(todayTotal),
      actionLabel: 'عرض المبيعات',
      actionTo: '/invoices',
      icon: Sparkles,
      roles: ['manager', 'admin'],
      value: todayTotal,
      createdAt: ctx.today,
    },
  ];
};

// 20 — Expiring batches nearing but not yet in the expiry alert window: "check-in reminder" (storekeeper) —
// reused as the 20th rule per the doc's "Setup"/"good news" style optional slot: here we surface
// suppliers with invoices missing the supplier invoice number/date after receiving (§1 "warning banner"),
// which the storekeeper/accountant needs to chase but has no dedicated screen surfacing it today.
const missingSupplierInvoiceRule: InsightRule = (ctx) => {
  const missing = db.purchaseOrders.filter((po) => po.status === 'RECEIVED' && !po.supplierInvoiceNo);
  if (!missing.length) return [];
  return [
    {
      id: 'missing-supplier-invoice:all',
      ruleKey: 'missing-supplier-invoice',
      severity: 'info',
      message: `${missing.length} أمر شراء مستلم بلا رقم فاتورة مورد`,
      metric: `${formatNumber(missing.length)} أمر`,
      actionLabel: 'عرض المشتريات',
      actionTo: '/purchases',
      icon: AlertTriangle,
      roles: ['accountant', 'manager', 'admin'],
      value: missing.length * 20,
      createdAt: ctx.today,
    },
  ];
};

export const INSIGHT_RULES: InsightRule[] = [
  reorderRule,
  deadStockRule,
  expiringRule,
  overdueCustomersRule,
  creditLimitRule,
  supplierDuesRule,
  vatDeadlineRule,
  cashInDrawerRule,
  shiftNotClosedRule,
  unsettledCardsRule,
  belowCostRule,
  discountLeakRule,
  refundSpikeRule,
  recurringJournalDueRule,
  recurringExpenseDueRule,
  budgetRule,
  openingBalanceEquityRule,
  backupOverdueRule,
  yearEndRule,
  goodNewsRule,
  missingSupplierInvoiceRule,
];

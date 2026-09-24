import { db, delay, localDateKey, round2, sum } from '@/mocks';
import { formatDate } from '@/modules/core/helpers/format';

/**
 * v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part C "/analytics"). Depth lives here —
 * a few solid tabs rather than forcing exhaustive coverage (see this phase's report for what was
 * skipped). Every chart pairs with a one-sentence, plain-Arabic auto-generated insight.
 *
 * Scope built: المبيعات (sales trend + weekday mix + payment methods), المنتجات (top/bottom by
 * profit), العملاء (new vs returning + revenue concentration). Skipped this phase: الفروع (branch
 * comparisons — needs Phase 9's branch data wired through every mock query, a bigger lift than one
 * tab), الربحية waterfall (needs the full expense-by-group breakdown Phase 12/reports owns) —
 * both are natural follow-ups once those pieces stabilize.
 */

const WEEKDAY_LABELS = ['الأحد', 'الاثنين', 'الثلاثاء', 'الأربعاء', 'الخميس', 'الجمعة', 'السبت'];

function soldInvoices() {
  return db.invoices.filter((i) => i.status !== 'DRAFT');
}

// --- المبيعات (sales) -----------------------------------------------------------------------------

export interface SalesTrendPoint {
  date: string;
  total: number;
}

export interface SalesAnalytics {
  trend: SalesTrendPoint[];
  trendInsight: string;
  byWeekday: { day: string; total: number; avg: number }[];
  weekdayInsight: string;
  paymentMix: { method: string; total: number; pct: number }[];
  avgInvoice: number;
  avgItemsPerInvoice: number;
  returnsRatePct: number;
}

export async function getSalesAnalytics(days = 30): Promise<SalesAnalytics> {
  await delay();
  const today = new Date();
  const trend: SalesTrendPoint[] = [];
  for (let d = days - 1; d >= 0; d--) {
    const day = new Date(today);
    day.setDate(day.getDate() - d);
    const key = localDateKey(day);
    const gross = sum(soldInvoices().filter((i) => localDateKey(i.date) === key), (i) => i.grandTotal);
    const refunds = sum(db.refunds.filter((r) => localDateKey(r.date) === key), (r) => r.grandTotal);
    trend.push({ date: key, total: round2(gross - refunds) });
  }
  const avgTotal = trend.reduce((a, t) => a + t.total, 0) / (trend.length || 1);
  const bestDay = [...trend].sort((a, b) => b.total - a.total)[0];
  const trendInsight =
    bestDay && avgTotal > 0
      ? `أفضل يوم كان ${formatDate(bestDay.date)} بمبيعات ${round2(bestDay.total)} ر.س — أعلى بنسبة ${round2(((bestDay.total - avgTotal) / avgTotal) * 100)}% من المتوسط`
      : 'لا توجد بيانات كافية بعد لهذه الفترة';

  const cutoff = new Date(today);
  cutoff.setDate(cutoff.getDate() - days);
  const cutoffKey = localDateKey(cutoff);
  const recent = soldInvoices().filter((i) => localDateKey(i.date) >= cutoffKey);

  const weekdayTotals = new Array(7).fill(0);
  const weekdayCounts = new Array(7).fill(0);
  for (const inv of recent) {
    const dow = new Date(inv.date).getDay();
    weekdayTotals[dow] += inv.grandTotal;
    weekdayCounts[dow] += 1;
  }
  const byWeekday = WEEKDAY_LABELS.map((day, i) => ({
    day,
    total: round2(weekdayTotals[i]),
    avg: weekdayCounts[i] > 0 ? round2(weekdayTotals[i] / weekdayCounts[i]) : 0,
  }));
  const bestWeekday = [...byWeekday].sort((a, b) => b.avg - a.avg)[0];
  const restAvg = byWeekday.filter((w) => w.day !== bestWeekday?.day).reduce((a, w) => a + w.avg, 0) / 6;
  const weekdayInsight =
    bestWeekday && bestWeekday.avg > 0 && restAvg > 0
      ? `${bestWeekday.day} هو أعلى يوم مبيعاً بمتوسط ${formatShort(bestWeekday.avg)} ر.س — أعلى بـ ${round2(((bestWeekday.avg - restAvg) / restAvg) * 100)}% من بقية الأيام`
      : 'لا توجد بيانات كافية بعد';

  const byMethod = new Map<string, number>();
  for (const inv of recent) {
    for (const t of inv.tenders ?? []) {
      const method = db.paymentMethods.find((m) => m.id === t.paymentMethodId);
      const label = method?.name ?? 'أخرى';
      byMethod.set(label, round2((byMethod.get(label) ?? 0) + t.amount));
    }
  }
  const methodTotal = [...byMethod.values()].reduce((a, v) => a + v, 0) || 1;
  const paymentMix = [...byMethod.entries()]
    .map(([method, total]) => ({ method, total, pct: round2((total / methodTotal) * 100) }))
    .sort((a, b) => b.total - a.total);

  const totalItems = sum(recent, (i) => sum(i.lines, (l) => l.qty));
  const avgInvoice = recent.length ? round2(sum(recent, (i) => i.grandTotal) / recent.length) : 0;
  const avgItemsPerInvoice = recent.length ? round2(totalItems / recent.length) : 0;
  const recentRefunds = db.refunds.filter((r) => localDateKey(r.date) >= cutoffKey);
  const returnsRatePct = recent.length ? round2((recentRefunds.length / recent.length) * 100) : 0;

  return { trend, trendInsight, byWeekday, weekdayInsight, paymentMix, avgInvoice, avgItemsPerInvoice, returnsRatePct };
}

function formatShort(v: number) {
  return v >= 1000 ? `${round2(v / 1000)}K` : `${round2(v)}`;
}

// --- المنتجات (products) --------------------------------------------------------------------------

export interface ProductProfitRow {
  id: string;
  name: string;
  sku: string;
  qty: number;
  revenue: number;
  grossProfit: number;
  marginPct: number;
}

export interface ProductAnalytics {
  top: ProductProfitRow[];
  bottom: ProductProfitRow[];
  insight: string;
}

export async function getProductAnalytics(days = 30, limit = 8): Promise<ProductAnalytics> {
  await delay();
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - days);
  const cutoffKey = localDateKey(cutoff);
  const recent = soldInvoices().filter((i) => localDateKey(i.date) >= cutoffKey);

  const byProduct = new Map<string, { qty: number; revenue: number; profit: number }>();
  for (const inv of recent) {
    for (const line of inv.lines) {
      const net = line.net ?? round2(line.qty * line.price - line.discount);
      const entry = byProduct.get(line.productId) ?? { qty: 0, revenue: 0, profit: 0 };
      entry.qty += line.qty;
      entry.revenue += net;
      entry.profit += net - line.qty * line.costPrice;
      byProduct.set(line.productId, entry);
    }
  }
  const rows: ProductProfitRow[] = [...byProduct.entries()].map(([productId, v]) => {
    const p = db.products.find((x) => x.id === productId);
    return {
      id: productId,
      name: p?.name ?? '—',
      sku: p?.sku ?? '',
      qty: round2(v.qty),
      revenue: round2(v.revenue),
      grossProfit: round2(v.profit),
      marginPct: v.revenue > 0 ? round2((v.profit / v.revenue) * 100) : 0,
    };
  });
  const sorted = [...rows].sort((a, b) => b.grossProfit - a.grossProfit);
  const top = sorted.slice(0, limit);
  const bottom = sorted.slice(-limit).reverse();
  const insight = top.length
    ? `"${top[0].name}" هو الأعلى ربحاً بإجمالي ${round2(top[0].grossProfit)} ر.س خلال آخر ${days} يوماً`
    : 'لا توجد مبيعات كافية بعد لهذه الفترة';

  return { top, bottom, insight };
}

// --- العملاء (customers) --------------------------------------------------------------------------

export interface CustomerAnalytics {
  newCount: number;
  returningCount: number;
  segmentInsight: string;
  topCustomers: { id: string; name: string; total: number }[];
  top10SharePct: number;
  concentrationInsight: string;
}

export async function getCustomerAnalytics(days = 30, limit = 8): Promise<CustomerAnalytics> {
  await delay();
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - days);
  const cutoffKey = localDateKey(cutoff);
  const recent = soldInvoices().filter((i) => localDateKey(i.date) >= cutoffKey && i.customerId);

  const firstSaleByCustomer = new Map<string, string>();
  for (const inv of soldInvoices()) {
    if (!inv.customerId) continue;
    const prev = firstSaleByCustomer.get(inv.customerId);
    if (!prev || inv.date < prev) firstSaleByCustomer.set(inv.customerId, inv.date);
  }
  const recentCustomerIds = new Set(recent.map((i) => i.customerId!));
  let newCount = 0;
  let returningCount = 0;
  for (const id of recentCustomerIds) {
    const firstSale = firstSaleByCustomer.get(id);
    if (firstSale && firstSale >= cutoffKey) newCount++;
    else returningCount++;
  }
  const segmentInsight =
    recentCustomerIds.size > 0
      ? `${round2((newCount / recentCustomerIds.size) * 100)}% من العملاء النشطين هذه الفترة عملاء جدد`
      : 'لا توجد بيانات كافية بعد';

  const byCustomer = new Map<string, number>();
  for (const inv of recent) byCustomer.set(inv.customerId!, round2((byCustomer.get(inv.customerId!) ?? 0) + inv.grandTotal));
  const sorted = [...byCustomer.entries()].map(([id, total]) => ({ id, name: db.customers.find((c) => c.id === id)?.name ?? '—', total })).sort((a, b) => b.total - a.total);
  const totalRevenue = sorted.reduce((a, c) => a + c.total, 0) || 1;
  const top10 = sorted.slice(0, 10);
  const top10SharePct = round2((top10.reduce((a, c) => a + c.total, 0) / totalRevenue) * 100);
  const concentrationInsight = sorted.length >= 10 ? `أفضل 10 عملاء يمثلون ${top10SharePct}% من إجمالي الإيرادات` : 'عدد العملاء غير كافٍ لحساب التركّز بدقة';

  return { newCount, returningCount, segmentInsight, topCustomers: sorted.slice(0, limit), top10SharePct, concentrationInsight };
}

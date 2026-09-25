<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { count, heading, kpis as printKpis, money, pct, qty, row, table as printTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
import { getSalesReport } from '../services/reportService';
import type { SalesReport } from '../types';

type View = 'product' | 'category' | 'day' | 'method' | 'cashier';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getSalesReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

// v2 (docs/v2/13 §1 "insights box"): the plain per-page summary; `ruleKeys` below (passed to
// ReportShell) additionally surfaces real insight-engine hits relevant to sales.
const insights = computed(() => {
  const s = data.value?.summary;
  const topProduct = data.value?.byProduct[0];
  if (!s) return null;
  const metrics = [{ label: 'متوسط الفاتورة', value: formatNumber(s.averageInvoice) }, { label: 'هامش الربح', value: `${s.netSales ? formatNumber((s.grossProfit / s.netSales) * 100, 1) : 0}%` }];
  if (topProduct) metrics.push({ label: 'أعلى صنف مبيعاً', value: topProduct.name });
  return { headline: `${formatNumber(s.invoiceCount)} فاتورة بإجمالي ${formatNumber(s.total)} خلال الفترة`, metrics };
});

const view = ref<View>('product');
const s = computed(() => data.value?.summary);

const kpis = computed(() =>
  s.value
    ? [
        { label: 'عدد الفواتير', value: s.value.invoiceCount, money: false },
        { label: 'صافي المبيعات (قبل الضريبة)', value: s.value.netSales, money: true },
        { label: 'الإجمالي شامل الضريبة', value: s.value.total, money: true },
        { label: 'المرتجعات', value: s.value.refunds, money: true },
        { label: 'تكلفة البضاعة المباعة', value: s.value.cogs, money: true },
        { label: 'مجمل الربح', value: s.value.grossProfit, money: true },
        { label: 'متوسط الفاتورة', value: s.value.averageInvoice, money: true },
        { label: 'الخصومات', value: s.value.discounts, money: true },
      ]
    : [],
);

const productColumns: Column<SalesReport['byProduct'][number]>[] = [
  { key: 'name', label: 'المنتج', sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'revenue', label: 'المبيعات (قبل الضريبة)', numeric: true, sortable: true },
  { key: 'cost', label: 'التكلفة', numeric: true, sortable: true },
  { key: 'profit', label: 'الربح', numeric: true, sortable: true },
];

/** Generic rows for the other breakdowns so one table renders them all. */
const simpleRows = computed(() => {
  const d = data.value;
  if (!d) return [];
  if (view.value === 'category') return d.byCategory.map((r) => ({ id: r.name, label: r.name, count: r.qty, total: r.revenue }));
  if (view.value === 'day') return [...d.byDay].reverse().map((r) => ({ id: r.date, label: formatDate(r.date), count: r.invoices, total: r.total }));
  if (view.value === 'method') return d.byMethod.map((r) => ({ id: r.method, label: r.method, count: r.count, total: r.total }));
  return d.byCashier.map((r) => ({ id: r.name, label: r.name, count: r.count, total: r.total }));
});
const simpleLabels: Record<Exclude<View, 'product'>, [string, string, string]> = {
  category: ['التصنيف', 'الكمية', 'المبيعات (قبل الضريبة)'],
  day: ['اليوم', 'الفواتير', 'الإجمالي شامل الضريبة'],
  method: ['طريقة الدفع', 'الفواتير', 'الإجمالي شامل الضريبة'],
  cashier: ['الكاشير', 'الفواتير', 'الإجمالي شامل الضريبة'],
};
const simpleColumns = computed<Column[]>(() => {
  const [a, b, c] = simpleLabels[view.value as Exclude<View, 'product'>] ?? ['', '', ''];
  return [
    { key: 'label', label: a },
    { key: 'count', label: b, numeric: true, sortable: true },
    { key: 'total', label: c, numeric: true, sortable: true },
  ];
});

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  if (view.value === 'product') {
    return { title: 'تقرير المبيعات حسب المنتج', columns: productColumns.map((c) => c.label), rows: d.byProduct.map((r) => [r.name, r.qty, r.revenue, r.cost, r.profit]) };
  }
  return { title: 'تقرير المبيعات', columns: simpleColumns.value.map((c) => c.label), rows: simpleRows.value.map((r) => [r.label, r.count, r.total]) };
});

const VIEW_LABEL: Record<View, string> = { product: 'حسب المنتج', category: 'حسب التصنيف', day: 'حسب اليوم', method: 'حسب طريقة الدفع', cashier: 'حسب الكاشير' };

// Official print (reference `salesReport.hbs`: KPI bar, the breakdown with a totals row, then ratios).
const print = computed<ReportPrintSpec | null>(() => {
  const d = data.value;
  const sum = s.value;
  if (!d || !sum) return null;
  const breakdown =
    view.value === 'product'
      ? printTable(
          [
            { label: '#', dim: true, align: 'center', width: 0.4 },
            { label: 'المنتج', width: 3 },
            { label: 'الكمية', numeric: true, width: 0.9 },
            { label: 'المبيعات (قبل الضريبة)', numeric: true, width: 1.4 },
            { label: 'التكلفة', numeric: true, width: 1.3 },
            { label: 'الربح', numeric: true, width: 1.3 },
          ],
          [
            ...d.byProduct.map((r, i) => row([count(i + 1), r.name, qty(r.qty), money(r.revenue), money(r.cost), money(r.profit)])),
            row(['', 'الإجمالي', qty(d.byProduct.reduce((a, r) => a + r.qty, 0)), money(d.byProduct.reduce((a, r) => a + r.revenue, 0)), money(d.byProduct.reduce((a, r) => a + r.cost, 0)), money(d.byProduct.reduce((a, r) => a + r.profit, 0))], 'total'),
          ],
          'لا توجد مبيعات في هذه الفترة',
        )
      : printTable(
          [
            { label: '#', dim: true, align: 'center', width: 0.4 },
            { label: simpleColumns.value[0].label, width: 3 },
            { label: simpleColumns.value[1].label, numeric: true, width: 1 },
            { label: simpleColumns.value[2].label, numeric: true, width: 1.5 },
          ],
          [
            ...simpleRows.value.map((r, i) => row([count(i + 1), r.label, view.value === 'category' ? qty(r.count) : count(r.count), money(r.total)])),
            row(['', 'الإجمالي', view.value === 'category' ? qty(simpleRows.value.reduce((a, r) => a + r.count, 0)) : count(simpleRows.value.reduce((a, r) => a + r.count, 0)), money(simpleRows.value.reduce((a, r) => a + r.total, 0))], 'total'),
          ],
          'لا توجد مبيعات في هذه الفترة',
        );
  return {
    meta: [{ label: 'التفصيل', value: VIEW_LABEL[view.value] }],
    blocks: [
      printKpis([
        { label: 'عدد الفواتير', value: count(sum.invoiceCount) },
        { label: 'صافي المبيعات (قبل الضريبة)', value: money(sum.netSales), emphasis: true },
        { label: 'ضريبة القيمة المضافة', value: money(sum.vat) },
        { label: 'الإجمالي شامل الضريبة', value: money(sum.total) },
      ]),
      printKpis([
        { label: 'المرتجعات', value: money(sum.refunds) },
        { label: 'الخصومات', value: money(sum.discounts) },
        { label: 'تكلفة البضاعة المباعة', value: money(sum.cogs) },
        { label: 'مجمل الربح', value: money(sum.grossProfit), emphasis: true },
      ]),
      heading(`تفاصيل المبيعات — ${VIEW_LABEL[view.value]}`),
      breakdown,
      heading('المؤشرات'),
      printTable(
        [
          { label: 'المؤشر', width: 3 },
          { label: 'القيمة', numeric: true, width: 1.4 },
        ],
        [
          row(['متوسط الفاتورة', money(sum.averageInvoice)]),
          row(['هامش مجمل الربح', pct(sum.netSales ? (sum.grossProfit / sum.netSales) * 100 : 0)]),
          row(['نسبة المرتجعات من المبيعات', pct(sum.total ? (sum.refunds / sum.total) * 100 : 0)]),
          row(['نسبة الخصومات من إجمالي المبيعات', pct(sum.grossSales ? (sum.discounts / sum.grossSales) * 100 : 0)]),
          row(['صافي المبيعات بعد المرتجعات', money(sum.netAfterRefunds)], 'subtotal'),
        ],
      ),
    ],
  };
});
</script>

<template>
  <ReportShell
    title="تقرير المبيعات"
    subtitle="أداء المبيعات للفترة"
    :from="from"
    :to="to"
    :loading="(loading || !ready) && !data"
    :error="error"
    :table="table"
    :print="print"
    :insights="insights"
    :rule-keys="['discount-leak', 'refund-spike', 'good-news']"
    @retry="reload"
  >
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div class="mb-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <div v-for="k in kpis" :key="k.label" class="rounded-xl border border-border bg-surface p-3.5">
        <p class="text-xs text-text-secondary">{{ k.label }}</p>
        <p class="mt-1 text-lg font-semibold">
          <MoneyText v-if="k.money" :value="k.value" />
          <span v-else class="num">{{ formatNumber(k.value) }}</span>
        </p>
      </div>
    </div>

    <div class="no-print mb-3">
      <SegmentedControl
        v-model="view"
        :options="[
          { value: 'product', label: 'حسب المنتج' },
          { value: 'category', label: 'حسب التصنيف' },
          { value: 'day', label: 'حسب اليوم' },
          { value: 'method', label: 'طريقة الدفع' },
          { value: 'cashier', label: 'الكاشير' },
        ]"
      />
    </div>

    <DataTable v-if="view === 'product'" :columns="productColumns" :rows="data?.byProduct" row-key="productId" :page-size="0" empty-title="لا توجد مبيعات في هذه الفترة">
      <template #cell-name="{ row }"><RouterLink :to="`/products/${row.productId}`" class="hover:text-primary">{{ row.name }}</RouterLink></template>
      <template #cell-revenue="{ row }"><MoneyText :value="row.revenue" plain /></template>
      <template #cell-cost="{ row }"><MoneyText :value="row.cost" plain class="text-text-secondary" /></template>
      <template #cell-profit="{ row }"><MoneyText :value="row.profit" plain signed /></template>
    </DataTable>
    <DataTable v-else :columns="simpleColumns" :rows="simpleRows" :page-size="0" empty-title="لا توجد مبيعات في هذه الفترة">
      <template #cell-total="{ row }"><MoneyText :value="row.total" /></template>
    </DataTable>
  </ReportShell>
</template>

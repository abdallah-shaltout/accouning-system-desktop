<script setup lang="ts">
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ComparisonToggle from '../components/ComparisonToggle.vue';
import DimensionFilters from '../components/DimensionFilters.vue';
import ReportShell from '../components/ReportShell.vue';
import StatementSection from '../components/StatementSection.vue';
import { useReportFilters } from '../controllers/useReportFilters';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getProfitAndLoss, getProfitAndLossComparison } from '../services/reportService';
import type { ProfitAndLoss } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { branchId, costCenterId, currency, comparison, branches, costCenters, currencies, showBranch, showCostCenter, showCurrency, dimensionQuery, syncDimensionsUrl, compareRange } =
  useReportFilters();

const compare = computed(() => compareRange(from.value, to.value));

async function load(): Promise<{ data: ProfitAndLoss; previous?: ProfitAndLoss }> {
  const range = { from: from.value || undefined, to: to.value || undefined, ...dimensionQuery.value };
  if (comparison.value === 'none') return { data: await getProfitAndLoss(range) };
  const { current, previous } = await getProfitAndLossComparison(range, compare.value);
  return { data: current, previous };
}

const { data: wrapped, loading, error, reload } = useAsync(load, { immediate: false });
const data = computed(() => wrapped.value?.data);
const previous = computed(() => wrapped.value?.previous);

watch([from, to, branchId, costCenterId, currency, comparison, ready], () => {
  if (!ready.value) return;
  syncUrl();
  syncDimensionsUrl();
  reload();
});

const margin = computed(() => (data.value && data.value.netRevenue ? (data.value.netIncome / data.value.netRevenue) * 100 : 0));
const grossMargin = computed(() => (data.value && data.value.netRevenue ? (data.value.grossProfit / data.value.netRevenue) * 100 : 0));

const deltaNetIncome = computed(() => (data.value && previous.value ? round2Local(data.value.netIncome - previous.value.netIncome) : 0));
const deltaNetIncomePct = computed(() => (previous.value?.netIncome ? round2Local((deltaNetIncome.value / Math.abs(previous.value.netIncome)) * 100) : 0));
function round2Local(n: number) {
  return Math.round(n * 100) / 100;
}

const insights = computed(() => {
  if (!data.value) return null;
  const metrics = [
    { label: 'هامش مجمل الربح', value: `${formatNumber(grossMargin.value, 1)}%` },
    { label: 'هامش صافي الربح', value: `${formatNumber(margin.value, 1)}%` },
  ];
  if (previous.value) metrics.push({ label: 'التغير في صافي الربح', value: `${deltaNetIncome.value >= 0 ? '+' : ''}${formatNumber(deltaNetIncomePct.value, 1)}%` });
  return {
    headline: data.value.netIncome >= 0 ? `ربح صافٍ قدره ${formatNumber(data.value.netIncome)} للفترة` : `خسارة صافية قدرها ${formatNumber(Math.abs(data.value.netIncome))} للفترة`,
    metrics,
  };
});

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  const rows: (string | number)[][] = [];
  const prev = previous.value;
  const withCompare = (row: (string | number)[], amount: number, prevAmount?: number) => (prev ? [...row, prevAmount ?? '', prevAmount !== undefined ? round2Local(amount - prevAmount) : ''] : row);
  const findPrev = (list: typeof d.revenue | undefined, code: string) => list?.find((l) => l.code === code)?.amount;
  const section = (title: string, lines: typeof d.revenue, total: number, prevLines?: typeof d.revenue, prevTotal?: number) => {
    rows.push(withCompare([title, '', ''], 0));
    for (const l of lines) rows.push(withCompare(['', `${l.code} ${l.name}`, l.amount], l.amount, findPrev(prevLines, l.code)));
    rows.push(withCompare(['', `إجمالي ${title}`, total], total, prevTotal));
  };
  section('الإيرادات', d.revenue, d.netRevenue, prev?.revenue, prev?.netRevenue);
  section('تكلفة المبيعات', d.cogs, d.totalCogs, prev?.cogs, prev?.totalCogs);
  rows.push(withCompare(['مجمل الربح', '', d.grossProfit], d.grossProfit, prev?.grossProfit));
  section('المصروفات', d.expenses, d.totalExpenses, prev?.expenses, prev?.totalExpenses);
  rows.push(withCompare(['صافي الربح', '', d.netIncome], d.netIncome, prev?.netIncome));
  const columns = prev ? ['البند', 'الحساب', 'المبلغ', 'الفترة السابقة', 'الفرق'] : ['البند', 'الحساب', 'المبلغ'];
  return { title: 'قائمة الدخل', columns, rows };
});
</script>

<template>
  <ReportShell
    title="قائمة الدخل"
    subtitle="الأرباح والخسائر للفترة المحددة"
    :from="from"
    :to="to"
    :loading="(loading || !ready) && !data"
    :error="error"
    :table="table"
    :insights="insights"
    @retry="reload"
  >
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
      <DimensionFilters
        v-model:branch-id="branchId"
        v-model:cost-center-id="costCenterId"
        v-model:currency="currency"
        :show-branch="showBranch"
        :show-cost-center="showCostCenter"
        :show-currency="showCurrency"
        :branches="branches"
        :cost-centers="costCenters"
        :currencies="currencies"
      />
      <ComparisonToggle v-model="comparison" />
    </template>

    <div v-if="data" class="grid items-start gap-5 xl:grid-cols-[1fr_300px]">
      <div class="space-y-4">
        <StatementSection title="الإيرادات" :lines="data.revenue" :total="data.netRevenue" total-label="صافي الإيرادات" />
        <StatementSection title="تكلفة المبيعات" :lines="data.cogs" :total="data.totalCogs" />
        <div class="flex items-center justify-between rounded-xl border border-border px-4 py-3 text-ui font-semibold">
          <span>مجمل الربح</span>
          <MoneyText :value="data.grossProfit" signed />
        </div>
        <StatementSection title="المصروفات" :lines="data.expenses" :total="data.totalExpenses" />
        <div class="flex items-center justify-between rounded-xl border-2 px-4 py-3.5 text-lead font-semibold" :class="data.netIncome >= 0 ? 'border-success/50' : 'border-danger/50'">
          <span>{{ data.netIncome >= 0 ? 'صافي الربح' : 'صافي الخسارة' }}</span>
          <MoneyText :value="data.netIncome" signed />
        </div>
      </div>
      <div class="no-print space-y-3">
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">هامش الربح الإجمالي</p>
          <p class="num mt-1 text-xl font-semibold">{{ formatNumber(grossMargin, 1) }}%</p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">هامش صافي الربح</p>
          <p class="num mt-1 text-xl font-semibold" :class="margin < 0 ? 'text-danger' : ''">{{ formatNumber(margin, 1) }}%</p>
        </div>
        <div v-if="previous" class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">التغير في صافي الربح عن {{ comparison === 'sameLastYear' ? 'نفس الفترة العام الماضي' : 'الفترة السابقة' }}</p>
          <p class="num mt-1 text-xl font-semibold" :class="deltaNetIncome < 0 ? 'text-danger' : 'text-success'">
            {{ deltaNetIncome >= 0 ? '+' : '' }}{{ formatNumber(deltaNetIncomePct, 1) }}%
          </p>
        </div>
        <p class="text-tiny leading-5 text-text-secondary">اضغط على أي حساب لعرض كشف الحساب المفصل.</p>
      </div>
    </div>
  </ReportShell>
</template>

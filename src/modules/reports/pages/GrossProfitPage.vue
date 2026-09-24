<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Gross profit: per invoice/product/category: sales − COGS, margin %"). */
import { computed, ref, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getGrossProfitReport } from '../services/reportService';
import type { GrossProfitRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const groupBy = ref<'invoice' | 'product' | 'category'>('product');
const { data, loading, error, reload } = useAsync(() => getGrossProfitReport({ from: from.value || undefined, to: to.value || undefined }, groupBy.value), { immediate: false });
watch([from, to, groupBy, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const totals = computed(() => {
  const rows = data.value ?? [];
  const revenue = rows.reduce((a, r) => a + r.revenue, 0);
  const profit = rows.reduce((a, r) => a + r.profit, 0);
  return { revenue, cost: rows.reduce((a, r) => a + r.cost, 0), profit, marginPct: revenue > 0 ? (profit / revenue) * 100 : 0 };
});

const insights = computed(() => {
  if (!data.value?.length) return null;
  return {
    headline: `مجمل ربح ${formatNumber(totals.value.profit)} بهامش ${formatNumber(totals.value.marginPct, 1)}%`,
    metrics: [{ label: 'أعلى هامش', value: `${[...data.value].sort((a, b) => b.marginPct - a.marginPct)[0]?.label ?? ''}` }],
  };
});

const columns: Column<GrossProfitRow>[] = [
  { key: 'label', label: groupBy.value === 'invoice' ? 'الفاتورة' : groupBy.value === 'product' ? 'المنتج' : 'التصنيف', sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'revenue', label: 'المبيعات', numeric: true, sortable: true },
  { key: 'cost', label: 'التكلفة', numeric: true, sortable: true },
  { key: 'profit', label: 'الربح', numeric: true, sortable: true },
  { key: 'marginPct', label: 'الهامش %', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'تقرير مجمل الربح', columns: columns.map((c) => c.label), rows: data.value.map((r) => [r.label, r.qty, r.revenue, r.cost, r.profit, r.marginPct]) },
);
</script>

<template>
  <ReportShell title="تقرير مجمل الربح" subtitle="المبيعات ناقص تكلفة البضاعة المباعة، مع الهامش" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
      <SegmentedControl v-model="groupBy" :options="[{ value: 'product', label: 'حسب المنتج' }, { value: 'category', label: 'حسب التصنيف' }, { value: 'invoice', label: 'حسب الفاتورة' }]" />
    </template>

    <DataTable :columns="columns" :rows="data" row-key="key" :page-size="0">
      <template #cell-revenue="{ row }"><MoneyText :value="row.revenue" plain /></template>
      <template #cell-cost="{ row }"><MoneyText :value="row.cost" plain class="text-text-secondary" /></template>
      <template #cell-profit="{ row }"><MoneyText :value="row.profit" plain signed /></template>
      <template #cell-marginPct="{ row }"><span class="num" :class="row.marginPct < 0 ? 'text-danger' : ''">{{ formatNumber(row.marginPct, 1) }}%</span></template>
      <template #footer>
        <tr>
          <td class="px-3 py-3">الإجمالي</td>
          <td class="px-3 py-3" />
          <td class="px-3 py-3"><MoneyText :value="totals.revenue" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.cost" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.profit" signed /></td>
          <td class="px-3 py-3"><span class="num">{{ formatNumber(totals.marginPct, 1) }}%</span></td>
        </tr>
      </template>
    </DataTable>
  </ReportShell>
</template>

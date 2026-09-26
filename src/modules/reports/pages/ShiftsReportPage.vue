<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Shift / Z-report history: per shift, sales by method, expected vs counted, variance"). */
import { computed, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getShiftsReport } from '../services/reportService';
import type { ShiftReportRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getShiftsReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  if (!data.value?.length) return null;
  const totalVariance = data.value.reduce((a, r) => a + r.variance, 0);
  const shortages = data.value.filter((r) => r.variance < -0.01).length;
  return { headline: `${formatNumber(data.value.length)} وردية مغلقة خلال الفترة`, metrics: [{ label: 'إجمالي الفروقات', value: formatNumber(totalVariance) }, { label: 'ورديات بها عجز', value: String(shortages) }] };
});

const columns: Column<ShiftReportRow>[] = [
  { key: 'number', label: 'الوردية', sortable: true },
  { key: 'openedBy', label: 'الكاشير', sortable: true },
  { key: 'openedAt', label: 'الفتح', sortable: true },
  { key: 'closedAt', label: 'الإغلاق', sortable: true },
  { key: 'salesTotal', label: 'المبيعات', numeric: true, sortable: true },
  { key: 'expectedCash', label: 'المتوقع نقداً', numeric: true, sortable: true },
  { key: 'countedCash', label: 'المعدود', numeric: true, sortable: true },
  { key: 'variance', label: 'الفرق', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'سجل الورديات',
    columns: columns.map((c) => c.label),
    rows: data.value.map((r) => [r.number, r.openedBy, formatDateTime(r.openedAt), r.closedAt ? formatDateTime(r.closedAt) : '', r.salesTotal, r.expectedCash, r.countedCash, r.variance]),
  },
);
</script>

<template>
  <ReportShell title="سجل الورديات" subtitle="النقدية المتوقعة مقابل المعدودة لكل وردية مغلقة" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <DataTable :columns="columns" :rows="data" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'pos-shift-report', params: { id: r.id } })">
      <template #cell-openedAt="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.openedAt) }}</span></template>
      <template #cell-closedAt="{ row }"><span class="num text-text-secondary">{{ row.closedAt ? formatDateTime(row.closedAt) : '—' }}</span></template>
      <template #cell-salesTotal="{ row }"><MoneyText :value="row.salesTotal" plain /></template>
      <template #cell-expectedCash="{ row }"><MoneyText :value="row.expectedCash" plain /></template>
      <template #cell-countedCash="{ row }"><MoneyText :value="row.countedCash" plain /></template>
      <template #cell-variance="{ row }"><MoneyText :value="row.variance" plain signed /></template>
    </DataTable>
  </ReportShell>
</template>

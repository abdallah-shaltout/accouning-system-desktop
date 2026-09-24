<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Branch comparison: KPIs per branch"). */
import { computed, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getBranchComparison } from '../services/reportService';
import type { BranchComparisonRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getBranchComparison({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  if (!data.value?.length) return null;
  const top = data.value[0];
  return { headline: `${top.name} هو الفرع الأعلى مبيعاً بإجمالي ${formatNumber(top.sales)}`, metrics: [] };
});

const columns: Column<BranchComparisonRow>[] = [
  { key: 'name', label: 'الفرع', sortable: true },
  { key: 'sales', label: 'المبيعات', numeric: true, sortable: true },
  { key: 'grossProfit', label: 'مجمل الربح', numeric: true, sortable: true },
  { key: 'invoiceCount', label: 'عدد الفواتير', numeric: true, sortable: true },
  { key: 'averageInvoice', label: 'متوسط الفاتورة', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'مقارنة الفروع', columns: columns.map((c) => c.label), rows: data.value.map((r) => [r.name, r.sales, r.grossProfit, r.invoiceCount, r.averageInvoice]) },
);
</script>

<template>
  <ReportShell title="مقارنة الفروع" subtitle="أداء المبيعات والربحية لكل فرع خلال الفترة" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <DataTable :columns="columns" :rows="data" row-key="branchId" :page-size="0">
      <template #cell-sales="{ row }"><MoneyText :value="row.sales" plain /></template>
      <template #cell-grossProfit="{ row }"><MoneyText :value="row.grossProfit" plain signed /></template>
      <template #cell-averageInvoice="{ row }"><MoneyText :value="row.averageInvoice" plain /></template>
    </DataTable>
  </ReportShell>
</template>

<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Returns analysis: by reason, product, cashier; returns rate"). */
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
import { getReturnsReport } from '../services/reportService';
import type { ReturnsReportRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const view = ref<'byReason' | 'byProduct' | 'byCashier'>('byReason');
const { data, loading, error, reload } = useAsync(() => getReturnsReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const rows = computed<ReturnsReportRow[]>(() => (data.value ? data.value[view.value] : []));

const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  return {
    headline: `معدل الإرجاع ${formatNumber(d.returnRatePct, 1)}% من عدد الفواتير`,
    metrics: [{ label: 'عدد المرتجعات', value: String(d.totalRefunds) }, { label: 'أعلى سبب', value: d.byReason[0]?.label ?? '—' }],
  };
});

const columns: Column<ReturnsReportRow>[] = [
  { key: 'label', label: view.value === 'byReason' ? 'السبب' : view.value === 'byProduct' ? 'المنتج' : 'الكاشير', sortable: true },
  { key: 'count', label: 'عدد المرتجعات', numeric: true, sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'amount', label: 'القيمة', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() => (rows.value.length ? { title: 'تحليل المرتجعات', columns: columns.map((c) => c.label), rows: rows.value.map((r) => [r.label, r.count, r.qty, r.amount]) } : undefined));
</script>

<template>
  <ReportShell title="تحليل المرتجعات" subtitle="مرتجعات المبيعات حسب السبب والمنتج والكاشير" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
      <SegmentedControl v-model="view" :options="[{ value: 'byReason', label: 'حسب السبب' }, { value: 'byProduct', label: 'حسب المنتج' }, { value: 'byCashier', label: 'حسب الكاشير' }]" />
    </template>

    <div v-if="data" class="mb-4 grid gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">إجمالي الفواتير</p><p class="num mt-1 text-lg font-semibold">{{ formatNumber(data.totalInvoices) }}</p></div>
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">إجمالي المرتجعات</p><p class="num mt-1 text-lg font-semibold">{{ formatNumber(data.totalRefunds) }}</p></div>
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">معدل الإرجاع</p><p class="num mt-1 text-lg font-semibold">{{ formatNumber(data.returnRatePct, 1) }}%</p></div>
    </div>

    <DataTable :columns="columns" :rows="rows" row-key="key" :page-size="0">
      <template #cell-amount="{ row }"><MoneyText :value="row.amount" plain /></template>
    </DataTable>
  </ReportShell>
</template>

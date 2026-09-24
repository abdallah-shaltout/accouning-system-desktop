<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Discounts & price overrides: per cashier and product: list price vs charged"). */
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
import { getDiscountsReport } from '../services/reportService';
import type { DiscountReportRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const groupBy = ref<'cashier' | 'product'>('cashier');
const { data, loading, error, reload } = useAsync(() => getDiscountsReport({ from: from.value || undefined, to: to.value || undefined }, groupBy.value), { immediate: false });
watch([from, to, groupBy, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const totalDiscount = computed(() => (data.value ?? []).reduce((a, r) => a + r.discountValue, 0));
const insights = computed(() => {
  if (!data.value?.length) return null;
  const top = [...data.value].sort((a, b) => b.discountPct - a.discountPct)[0];
  return { headline: `إجمالي الخصومات ${formatNumber(totalDiscount.value)}`, metrics: [{ label: 'أعلى نسبة خصم', value: `${top?.label ?? ''} (${formatNumber(top?.discountPct ?? 0, 1)}%)` }] };
});

const columns: Column<DiscountReportRow>[] = [
  { key: 'label', label: groupBy.value === 'cashier' ? 'الكاشير' : 'المنتج', sortable: true },
  { key: 'invoiceCount', label: 'عدد الفواتير/الأسطر', numeric: true, sortable: true },
  { key: 'listValue', label: 'السعر المعلن', numeric: true, sortable: true },
  { key: 'chargedValue', label: 'السعر المحصّل', numeric: true, sortable: true },
  { key: 'discountValue', label: 'قيمة الخصم', numeric: true, sortable: true },
  { key: 'discountPct', label: 'نسبة الخصم %', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'الخصومات وتجاوزات السعر', columns: columns.map((c) => c.label), rows: data.value.map((r) => [r.label, r.invoiceCount, r.listValue, r.chargedValue, r.discountValue, r.discountPct]) },
);
</script>

<template>
  <ReportShell title="الخصومات وتجاوزات السعر" subtitle="السعر المعلن مقابل السعر المحصّل فعلياً" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
      <SegmentedControl v-model="groupBy" :options="[{ value: 'cashier', label: 'حسب الكاشير' }, { value: 'product', label: 'حسب المنتج' }]" />
    </template>

    <DataTable :columns="columns" :rows="data" row-key="key" :page-size="0">
      <template #cell-listValue="{ row }"><MoneyText :value="row.listValue" plain class="text-text-secondary" /></template>
      <template #cell-chargedValue="{ row }"><MoneyText :value="row.chargedValue" plain /></template>
      <template #cell-discountValue="{ row }"><MoneyText :value="row.discountValue" plain class="text-warning" /></template>
      <template #cell-discountPct="{ row }"><span class="num">{{ formatNumber(row.discountPct, 1) }}%</span></template>
    </DataTable>
  </ReportShell>
</template>

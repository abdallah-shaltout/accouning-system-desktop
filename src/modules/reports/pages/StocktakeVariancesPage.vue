<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Stocktake variances: per count, qty and value variance, by category"). */
import { computed } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getStocktakeVariances } from '../services/reportService';
import type { StocktakeVarianceRow } from '../types';

const { data, loading, error, reload } = useAsync(() => getStocktakeVariances());

const totals = computed(() => ({
  qty: (data.value ?? []).reduce((a, r) => a + r.qtyVariance, 0),
  value: (data.value ?? []).reduce((a, r) => a + r.valueVariance, 0),
}));

const byCategory = computed(() => {
  const map = new Map<string, number>();
  for (const r of data.value ?? []) map.set(r.category, (map.get(r.category) ?? 0) + r.valueVariance);
  return [...map.entries()].sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]));
});

const insights = computed(() => {
  if (!data.value?.length) return null;
  return { headline: `${formatNumber(data.value.length)} سطراً فيه فرق جرد`, metrics: [{ label: 'صافي فرق القيمة', value: formatNumber(totals.value.value) }] };
});

const columns: Column<StocktakeVarianceRow>[] = [
  { key: 'countNumber', label: 'الجرد', sortable: true },
  { key: 'name', label: 'الصنف', sortable: true },
  { key: 'category', label: 'التصنيف', sortable: true },
  { key: 'systemQty', label: 'الكمية بالنظام', numeric: true, sortable: true },
  { key: 'countedQty', label: 'الكمية المعدودة', numeric: true, sortable: true },
  { key: 'qtyVariance', label: 'فرق الكمية', numeric: true, sortable: true },
  { key: 'valueVariance', label: 'فرق القيمة', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'فروقات الجرد',
    columns: columns.map((c) => c.label),
    rows: data.value.map((r) => [r.countNumber, r.name, r.category, r.systemQty, r.countedQty, r.qtyVariance, r.valueVariance]),
  },
);
</script>

<template>
  <ReportShell title="فروقات الجرد" subtitle="الفرق بين الكمية بالنظام والكمية المعدودة فعلياً، لكل عمليات الجرد المكتملة" :loading="loading && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <div v-if="byCategory.length" class="mb-4 flex flex-wrap gap-2">
      <span v-for="[name, v] in byCategory" :key="name" class="rounded-full border border-border px-3 py-1 text-xs">
        {{ name }}: <MoneyText :value="v" plain signed class="font-medium" />
      </span>
    </div>

    <DataTable :columns="columns" :rows="data" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'count', params: { id: r.countId } })">
      <template #cell-qtyVariance="{ row }"><span class="num" :class="row.qtyVariance < 0 ? 'text-danger' : 'text-success'">{{ row.qtyVariance > 0 ? '+' : '' }}{{ formatNumber(row.qtyVariance) }}</span></template>
      <template #cell-valueVariance="{ row }"><MoneyText :value="row.valueVariance" plain signed /></template>
      <template #footer>
        <tr>
          <td class="px-3 py-3" colspan="5">الإجمالي</td>
          <td class="px-3 py-3"><span class="num">{{ formatNumber(totals.qty) }}</span></td>
          <td class="px-3 py-3"><MoneyText :value="totals.value" signed /></td>
        </tr>
      </template>
    </DataTable>
  </ReportShell>
</template>

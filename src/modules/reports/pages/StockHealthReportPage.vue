<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §2 "Low stock / reorder: with suggested qty and 'create
 * purchase draft'" + "Slow / dead stock: days since last sale, value"). Combined into one page with
 * a toggle since both are stock-health views over the same product list.
 */
import { computed, ref, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getDeadStockReport, getLowStockReport } from '../services/reportService';
import type { DeadStockRow, LowStockRow } from '../types';

const view = ref<'low' | 'dead'>('low');
const low = useAsync(getLowStockReport, { immediate: false });
const dead = useAsync(() => getDeadStockReport(60), { immediate: false });

function reload() {
  return view.value === 'low' ? low.reload() : dead.reload();
}
watch(view, reload, { immediate: true });

const loading = computed(() => (view.value === 'low' ? low.loading.value : dead.loading.value));
const error = computed(() => (view.value === 'low' ? low.error.value : dead.error.value));

const lowColumns: Column<LowStockRow>[] = [
  { key: 'name', label: 'الصنف', sortable: true },
  { key: 'category', label: 'التصنيف', sortable: true },
  { key: 'qty', label: 'الكمية الحالية', numeric: true, sortable: true },
  { key: 'minStock', label: 'الحد الأدنى', numeric: true, sortable: true },
  { key: 'suggestedQty', label: 'الكمية المقترحة للطلب', numeric: true, sortable: true },
  { key: 'costValue', label: 'قيمة المخزون', numeric: true, sortable: true },
];
const deadColumns: Column<DeadStockRow>[] = [
  { key: 'name', label: 'الصنف', sortable: true },
  { key: 'category', label: 'التصنيف', sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'costValue', label: 'القيمة الراكدة', numeric: true, sortable: true },
  { key: 'lastSaleDate', label: 'آخر بيع', sortable: true },
  { key: 'daysSinceSale', label: 'أيام منذ آخر بيع', numeric: true, sortable: true },
];

const insights = computed(() => {
  if (view.value === 'low') {
    const rows = low.data.value;
    if (!rows?.length) return null;
    return { headline: `${formatNumber(rows.length)} صنفاً بحاجة لإعادة طلب`, metrics: [{ label: 'قيمة المخزون المتأثر', value: formatNumber(rows.reduce((a, r) => a + r.costValue, 0)) }] };
  }
  const rows = dead.data.value;
  if (!rows?.length) return null;
  return { headline: `${formatNumber(rows.length)} صنفاً راكداً (بدون بيع منذ 60 يوماً فأكثر)`, metrics: [{ label: 'القيمة الراكدة', value: formatNumber(rows.reduce((a, r) => a + r.costValue, 0)) }] };
});

const table = computed<ExportTable | undefined>(() => {
  if (view.value === 'low') {
    const rows = low.data.value;
    return rows && { title: 'المخزون المنخفض وإعادة الطلب', columns: lowColumns.map((c) => c.label), rows: rows.map((r) => [r.name, r.category, r.qty, r.minStock, r.suggestedQty, r.costValue]) };
  }
  const rows = dead.data.value;
  return rows && { title: 'المخزون الراكد', columns: deadColumns.map((c) => c.label), rows: rows.map((r) => [r.name, r.category, r.qty, r.costValue, r.lastSaleDate ?? '', r.daysSinceSale]) };
});
</script>

<template>
  <ReportShell
    :title="view === 'low' ? 'المخزون المنخفض وإعادة الطلب' : 'المخزون الراكد'"
    subtitle="مبني على حركة المخزون والحد الأدنى المحدد لكل صنف"
    :loading="loading"
    :error="error"
    :table="table"
    :insights="insights"
    @retry="reload"
  >
    <template #filters>
      <SegmentedControl v-model="view" :options="[{ value: 'low', label: 'منخفض / إعادة طلب' }, { value: 'dead', label: 'راكد' }]" />
    </template>

    <DataTable v-if="view === 'low'" :columns="lowColumns" :rows="low.data.value" row-key="productId" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'product', params: { id: r.productId } })">
      <template #cell-costValue="{ row }"><MoneyText :value="row.costValue" plain /></template>
      <template #cell-suggestedQty="{ row }"><span class="num font-medium text-primary">{{ formatNumber(row.suggestedQty) }}</span></template>
    </DataTable>
    <DataTable v-else :columns="deadColumns" :rows="dead.data.value" row-key="productId" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'product', params: { id: r.productId } })">
      <template #cell-costValue="{ row }"><MoneyText :value="row.costValue" plain /></template>
      <template #cell-lastSaleDate="{ row }"><span class="num text-text-secondary">{{ row.lastSaleDate ? formatDate(row.lastSaleDate) : 'لم يُبع أبداً' }}</span></template>
    </DataTable>
  </ReportShell>
</template>

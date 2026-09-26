<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Purchases summary: by supplier/product/period; average purchase price trend"). */
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
import { getPurchasesReport } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const view = ref<'bySupplier' | 'byProduct'>('bySupplier');
const { data, loading, error, reload } = useAsync(() => getPurchasesReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  const s = data.value?.summary;
  if (!s) return null;
  return { headline: `${formatNumber(s.poCount)} أمر شراء بإجمالي ${formatNumber(s.total)}`, metrics: [{ label: 'المرتجعات', value: formatNumber(s.returns) }] };
});

const supplierColumns: Column<{ supplierId: string; name: string; count: number; total: number }>[] = [
  { key: 'name', label: 'المورد', sortable: true },
  { key: 'count', label: 'عدد الأوامر', numeric: true, sortable: true },
  { key: 'total', label: 'الإجمالي', numeric: true, sortable: true },
];
const productColumns: Column<{ productId: string; name: string; qty: number; total: number; avgPrice: number }>[] = [
  { key: 'name', label: 'المنتج', sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'total', label: 'الإجمالي', numeric: true, sortable: true },
  { key: 'avgPrice', label: 'متوسط سعر الشراء', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  if (view.value === 'bySupplier') return { title: 'المشتريات حسب المورد', columns: supplierColumns.map((c) => c.label), rows: d.bySupplier.map((r) => [r.name, r.count, r.total]) };
  return { title: 'المشتريات حسب المنتج', columns: productColumns.map((c) => c.label), rows: d.byProduct.map((r) => [r.name, r.qty, r.total, r.avgPrice]) };
});
</script>

<template>
  <ReportShell title="تقرير المشتريات" subtitle="ملخص المشتريات حسب المورد والمنتج" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
      <SegmentedControl v-model="view" :options="[{ value: 'bySupplier', label: 'حسب المورد' }, { value: 'byProduct', label: 'حسب المنتج' }]" />
    </template>

    <div v-if="data" class="mb-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">عدد أوامر الشراء</p><p class="num mt-1 text-lg font-semibold">{{ formatNumber(data.summary.poCount) }}</p></div>
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">إجمالي المشتريات</p><p class="mt-1 text-lg font-semibold"><MoneyText :value="data.summary.grossPurchases" /></p></div>
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">الضريبة</p><p class="mt-1 text-lg font-semibold"><MoneyText :value="data.summary.vat" /></p></div>
      <div class="rounded-xl border border-border bg-surface p-3.5"><p class="text-xs text-text-secondary">المرتجعات</p><p class="mt-1 text-lg font-semibold"><MoneyText :value="data.summary.returns" /></p></div>
    </div>

    <DataTable v-if="view === 'bySupplier'" :columns="supplierColumns" :rows="data?.bySupplier" row-key="supplierId" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'supplier', params: { id: r.supplierId } })">
      <template #cell-total="{ row }"><MoneyText :value="row.total" plain /></template>
    </DataTable>
    <DataTable v-else :columns="productColumns" :rows="data?.byProduct" row-key="productId" :page-size="0" clickable @row-click="(r) => $router.push({ name: 'product', params: { id: r.productId } })">
      <template #cell-total="{ row }"><MoneyText :value="row.total" plain /></template>
      <template #cell-avgPrice="{ row }"><MoneyText :value="row.avgPrice" plain class="text-text-secondary" /></template>
    </DataTable>
  </ReportShell>
</template>

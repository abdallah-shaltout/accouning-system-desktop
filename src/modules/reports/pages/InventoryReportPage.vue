<script setup lang="ts">
import { computed, ref } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber, todayKey } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getInventoryReport } from '../services/reportService';
import type { InventoryReportRow } from '../types';

const { data, loading, error, reload } = useAsync(getInventoryReport);
const filter = ref<'all' | 'low' | 'out'>('all');

const rows = computed(() => (data.value ?? []).filter((r) => filter.value === 'all' || (filter.value === 'low' ? r.status !== 'ok' : r.status === 'out')));
const totals = computed(() => ({
  qty: rows.value.reduce((a, r) => a + r.qty, 0),
  cost: rows.value.reduce((a, r) => a + r.costValue, 0),
  retail: rows.value.reduce((a, r) => a + r.retailValue, 0),
}));
const byCategory = computed(() => {
  const map = new Map<string, { qty: number; cost: number }>();
  for (const r of data.value ?? []) {
    const c = map.get(r.category) ?? { qty: 0, cost: 0 };
    c.qty += r.qty;
    c.cost += r.costValue;
    map.set(r.category, c);
  }
  return [...map.entries()].sort((a, b) => b[1].cost - a[1].cost);
});

const STATUS = { ok: { label: 'متوفر', tone: 'success' }, low: { label: 'منخفض', tone: 'warning' }, out: { label: 'نفد', tone: 'danger' } } as const;

const columns: Column<InventoryReportRow>[] = [
  { key: 'name', label: 'الصنف', sortable: true },
  { key: 'category', label: 'التصنيف', sortable: true },
  { key: 'qty', label: 'الكمية', numeric: true, sortable: true },
  { key: 'costPrice', label: 'تكلفة الوحدة', numeric: true, sortable: true },
  { key: 'costValue', label: 'القيمة بالتكلفة', numeric: true, sortable: true },
  { key: 'retailValue', label: 'القيمة بسعر البيع', numeric: true, sortable: true },
  { key: 'status', label: 'الحالة' },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'تقرير المخزون',
    columns: ['الصنف', 'SKU', 'التصنيف', 'الكمية', 'الحد الأدنى', 'تكلفة الوحدة', 'القيمة بالتكلفة', 'القيمة بسعر البيع', 'الحالة'],
    rows: [
      ...rows.value.map((r) => [r.name, r.sku, r.category, r.qty, r.minStock, r.costPrice, r.costValue, r.retailValue, STATUS[r.status].label]),
      ['الإجمالي', '', '', totals.value.qty, '', '', totals.value.cost, totals.value.retail, ''],
    ],
  },
);
</script>

<template>
  <ReportShell title="تقرير المخزون" subtitle="الكميات الحالية وقيمة المخزون" :as-of="todayKey()" :loading="loading && !data" :error="error" :table="table" @retry="reload">
    <div class="mb-5 grid gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-surface p-3.5">
        <p class="text-xs text-text-secondary">إجمالي القطع</p>
        <p class="num mt-1 text-lg font-semibold">{{ formatNumber(totals.qty) }}</p>
      </div>
      <div class="rounded-xl border border-border bg-surface p-3.5">
        <p class="text-xs text-text-secondary">قيمة المخزون بالتكلفة</p>
        <p class="mt-1 text-lg font-semibold"><MoneyText :value="totals.cost" /></p>
      </div>
      <div class="rounded-xl border border-border bg-surface p-3.5">
        <p class="text-xs text-text-secondary">القيمة البيعية المتوقعة</p>
        <p class="mt-1 text-lg font-semibold"><MoneyText :value="totals.retail" /></p>
      </div>
    </div>

    <div class="mb-5 flex flex-wrap gap-2">
      <span v-for="[name, v] in byCategory" :key="name" class="rounded-full border border-border px-3 py-1 text-xs">
        {{ name }}: <MoneyText :value="v.cost" plain class="font-medium" />
      </span>
    </div>

    <div class="no-print mb-3">
      <SegmentedControl
        v-model="filter"
        :options="[
          { value: 'all', label: 'كل الأصناف', count: data?.length },
          { value: 'low', label: 'منخفض أو نافد', count: data?.filter((r) => r.status !== 'ok').length },
          { value: 'out', label: 'نافد', count: data?.filter((r) => r.status === 'out').length },
        ]"
      />
    </div>

    <DataTable :columns="columns" :rows="rows" row-key="productId" :page-size="0">
      <template #cell-name="{ row }">
        <RouterLink :to="`/products/${row.productId}`" class="hover:text-primary">{{ row.name }}</RouterLink>
        <span class="num block text-[11px] text-text-secondary">{{ row.sku }}</span>
      </template>
      <template #cell-category="{ row }"><span class="text-text-secondary">{{ row.category }}</span></template>
      <template #cell-costPrice="{ row }"><MoneyText :value="row.costPrice" plain /></template>
      <template #cell-costValue="{ row }"><MoneyText :value="row.costValue" plain /></template>
      <template #cell-retailValue="{ row }"><MoneyText :value="row.retailValue" plain class="text-text-secondary" /></template>
      <template #cell-status="{ row }"><StatusBadge :tone="STATUS[row.status].tone" :label="STATUS[row.status].label" /></template>
      <template #footer>
        <tr>
          <td class="px-3 py-3" colspan="2">الإجمالي</td>
          <td class="px-3 py-3"><span class="num">{{ formatNumber(totals.qty) }}</span></td>
          <td />
          <td class="px-3 py-3"><MoneyText :value="totals.cost" /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.retail" /></td>
          <td />
        </tr>
      </template>
    </DataTable>
  </ReportShell>
</template>

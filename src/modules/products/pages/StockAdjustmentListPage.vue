<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ClipboardCheck, ClipboardList, PackageMinus, PackagePlus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import FilterBar from '@/modules/core/components/blocks/FilterBar.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber, toDateKey } from '@/modules/core/helpers/format';
import { ADJUSTMENT_TYPE } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { adjustmentValue, getStockAdjustments } from '../services/inventoryService';
import type { StockAdjustment, StockAdjustmentType } from '../types';

const router = useRouter();
const route = useRoute();
const auth = useAuthStore();

const type = computed(() => (typeof route.query.type === 'string' ? (route.query.type as StockAdjustmentType) : undefined));
const from = computed(() => (typeof route.query.from === 'string' ? route.query.from : ''));
const to = computed(() => (typeof route.query.to === 'string' ? route.query.to : ''));

const { data, loading, error, reload } = useAsync(() => getStockAdjustments());

const rows = computed(() =>
  (data.value ?? []).filter((a) => {
    const key = toDateKey(a.date);
    return (!type.value || a.type === type.value) && (!from.value || key >= from.value) && (!to.value || key <= to.value);
  }),
);

const typeOptions = (['STOCK_IN', 'LOSS', 'STOCKTAKE'] as StockAdjustmentType[]).map((t) => ({
  value: t,
  label: ADJUSTMENT_TYPE[t].label,
}));

const columns: Column<StockAdjustment>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'type', label: 'النوع' },
  { key: 'note', label: 'البيان' },
  { key: 'lines', label: 'الأصناف', numeric: true, sortValue: (a) => a.lines.length, sortable: true },
  { key: 'value', label: 'الأثر على المخزون', numeric: true, sortable: true, sortValue: adjustmentValue },
  { key: 'status', label: 'الحالة' },
];
</script>

<template>
  <ListPage title="تسويات المخزون" subtitle="إدخال بضاعة، إتلاف وفقد، والجرد الدوري — كل تسوية تنشئ قيداً محاسبياً">
    <template #actions>
      <AppButton v-if="auth.can('inventory', 'write')" :icon="PackageMinus" :to="{ name: 'adjustment-new', query: { type: 'LOSS' } }">إتلاف / فقد</AppButton>
      <AppButton v-if="auth.can('inventory', 'write')" :icon="ClipboardCheck" :to="{ name: 'adjustment-new', query: { type: 'STOCKTAKE' } }">جرد جديد</AppButton>
      <AppButton v-if="auth.can('inventory', 'write')" variant="primary" :icon="PackagePlus" :to="{ name: 'adjustment-new', query: { type: 'STOCK_IN' } }">إدخال مخزون</AppButton>
    </template>
    <template #filters>
      <FilterBar :filters="[{ key: 'type', label: 'النوع', options: typeOptions }]" date-range />
    </template>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ClipboardList"
      empty-title="لا توجد تسويات"
      @retry="reload"
      @row-click="(a) => router.push({ name: 'adjustment', params: { id: a.id } })"
    >
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.date) }}</span></template>
      <template #cell-type="{ row }"><StatusBadge :tone="ADJUSTMENT_TYPE[row.type].tone" :label="ADJUSTMENT_TYPE[row.type].label" /></template>
      <template #cell-note="{ row }"><span class="line-clamp-1 text-text-secondary">{{ row.note ?? '—' }}</span></template>
      <template #cell-lines="{ row }"><span class="num">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-value="{ row }"><MoneyText :value="adjustmentValue(row)" signed dash-zero /></template>
      <template #cell-status="{ row }">
        <StatusBadge :tone="row.status === 'COMPLETED' ? 'success' : 'neutral'" :label="row.status === 'COMPLETED' ? 'معتمدة' : 'مسودة'" />
      </template>
    </DataTable>
  </ListPage>
</template>

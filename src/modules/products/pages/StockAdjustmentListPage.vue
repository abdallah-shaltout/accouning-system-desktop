<script setup lang="ts">
import { computed, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ClipboardCheck, ClipboardList, PackageMinus, PackagePlus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber, toDateKey } from '@/modules/core/helpers/format';
import { ADJUSTMENT_TYPE } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { adjustmentValue, getStockAdjustments } from '../services/inventoryService';
import type { StockAdjustment, StockAdjustmentType } from '../types';

const router = useRouter();
const auth = useAuthStore();
const type = ref<StockAdjustmentType | 'all'>('all');
const from = ref('');
const to = ref('');

const { data, loading, error, reload } = useAsync(() => getStockAdjustments());

const rows = computed(() =>
  (data.value ?? []).filter((a) => {
    const key = toDateKey(a.date);
    return (type.value === 'all' || a.type === type.value) && (!from.value || key >= from.value) && (!to.value || key <= to.value);
  }),
);

const typeOptions = computed(() => [
  { value: 'all' as const, label: 'الكل', count: data.value?.length },
  ...(['STOCK_IN', 'LOSS', 'STOCKTAKE'] as StockAdjustmentType[]).map((t) => ({
    value: t,
    label: ADJUSTMENT_TYPE[t].label,
    count: data.value?.filter((a) => a.type === t).length,
  })),
]);

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
  <div>
    <PageHeader title="تسويات المخزون" subtitle="إدخال بضاعة، إتلاف وفقد، والجرد الدوري — كل تسوية تنشئ قيداً محاسبياً">
      <template v-if="auth.can('inventory', 'write')" #actions>
        <AppButton :icon="PackageMinus" to="/inventory/adjustments/new?type=LOSS">إتلاف / فقد</AppButton>
        <AppButton :icon="ClipboardCheck" to="/inventory/adjustments/new?type=STOCKTAKE">جرد جديد</AppButton>
        <AppButton variant="primary" :icon="PackagePlus" to="/inventory/adjustments/new?type=STOCK_IN">إدخال مخزون</AppButton>
      </template>
    </PageHeader>

    <div class="mb-3 flex flex-wrap items-center justify-between gap-3">
      <SegmentedControl v-model="type" :options="typeOptions" />
      <DateRangeFilter v-model:from="from" v-model:to="to" />
    </div>

    <DataTable
      :columns="columns"
      :rows="rows"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ClipboardList"
      empty-title="لا توجد تسويات"
      @retry="reload"
      @row-click="(a) => router.push(`/inventory/adjustments/${a.id}`)"
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
  </div>
</template>

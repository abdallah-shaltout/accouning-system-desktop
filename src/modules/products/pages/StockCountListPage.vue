<script setup lang="ts">
import { useRouter } from 'vue-router';
import { ClipboardCheck, Plus } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import ListPage from '@/modules/core/components/layouts/ListPage.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { getStockCounts } from '../services/inventoryService';
import type { StockCount } from '../types';

const router = useRouter();
const auth = useAuthStore();
const { data, loading, error, reload } = useAsync(getStockCounts);

const SCOPE_LABEL: Record<StockCount['scope'], string> = { all: 'كل الأصناف', category: 'تصنيف محدد', location: 'موقع محدد' };
const STATUS: Record<StockCount['status'], { label: string; tone: 'neutral' | 'warning' | 'success' }> = {
  OPEN: { label: 'جارٍ العد', tone: 'warning' },
  REVIEW: { label: 'قيد المراجعة', tone: 'neutral' },
  COMPLETED: { label: 'مكتمل', tone: 'success' },
};

const columns: Column<StockCount>[] = [
  { key: 'number', label: 'الرقم', sortable: true },
  { key: 'startedAt', label: 'بدأ في', type: 'date', sortable: true },
  { key: 'scope', label: 'النطاق' },
  { key: 'lines', label: 'الأصناف', numeric: true, sortValue: (c) => c.lines.length },
  { key: 'blind', label: 'أعمى' },
  { key: 'status', label: 'الحالة', type: 'status', statusOf: (v: StockCount['status']) => STATUS[v] },
];
</script>

<template>
  <ListPage
    title="الجرد"
    subtitle="جرد بنطاق محدد مع لقطة رصيد النظام لحظة البدء — عد أعمى واختياري بالمسح"
    :primary-action-label="auth.can('inventory', 'write') ? 'جرد جديد' : undefined"
    :primary-action-icon="Plus"
    primary-action-to="/inventory/counts/new"
  >
    <DataTable
      :columns="columns"
      :rows="data"
      :loading="loading"
      :error="error"
      clickable
      :empty-icon="ClipboardCheck"
      empty-title="لا توجد عمليات جرد"
      @retry="reload"
      @row-click="(c) => router.push(`/inventory/counts/${c.id}`)"
    >
      <template #cell-scope="{ row }">{{ SCOPE_LABEL[row.scope] }}<span v-if="row.location" class="text-text-secondary"> — {{ row.location }}</span></template>
      <template #cell-lines="{ row }"><span class="num">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-blind="{ row }"><StatusBadge v-if="row.blind" tone="primary" label="أعمى" :dot="false" /><span v-else class="text-text-secondary">—</span></template>
    </DataTable>
  </ListPage>
</template>

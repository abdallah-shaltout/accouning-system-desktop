<script setup lang="ts">
import { useRouter } from 'vue-router';
import { ClipboardCheck, Plus } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
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
  { key: 'startedAt', label: 'بدأ في', sortable: true },
  { key: 'scope', label: 'النطاق' },
  { key: 'lines', label: 'الأصناف', numeric: true, sortValue: (c) => c.lines.length },
  { key: 'blind', label: 'أعمى' },
  { key: 'status', label: 'الحالة' },
];
</script>

<template>
  <div>
    <PageHeader title="الجرد" subtitle="جرد بنطاق محدد مع لقطة رصيد النظام لحظة البدء — عد أعمى واختياري بالمسح">
      <template v-if="auth.can('inventory', 'write')" #actions>
        <AppButton variant="primary" :icon="Plus" to="/inventory/counts/new">جرد جديد</AppButton>
      </template>
    </PageHeader>

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
      <template #cell-startedAt="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.startedAt) }}</span></template>
      <template #cell-scope="{ row }">{{ SCOPE_LABEL[row.scope] }}<span v-if="row.location" class="text-text-secondary"> — {{ row.location }}</span></template>
      <template #cell-lines="{ row }"><span class="num">{{ formatNumber(row.lines.length) }}</span></template>
      <template #cell-blind="{ row }"><StatusBadge v-if="row.blind" tone="primary" label="أعمى" :dot="false" /><span v-else class="text-text-secondary">—</span></template>
      <template #cell-status="{ row }"><StatusBadge :tone="STATUS[row.status].tone" :label="STATUS[row.status].label" /></template>
    </DataTable>
  </div>
</template>

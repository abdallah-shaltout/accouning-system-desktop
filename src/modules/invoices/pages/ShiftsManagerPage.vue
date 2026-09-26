<script setup lang="ts">
/** v2 phase 7 (docs/v2/06-sales-and-pos.md §5 "Manager screen /pos/shifts: open and closed shifts with variances. A manager can force-close a shift and reprint a Z-report"). */
import { computed } from 'vue';
import { RotateCcw, Printer } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { forceClosePosShift, getShifts, type ShiftRow } from '../services/invoiceService';

const auth = useAuthStore();
const toast = useToast();
const confirmDialog = useConfirm();
const { data, loading, error, reload } = useAsync(() => getShifts());
const rows = computed(() => data.value ?? []);

const columns: Column<ShiftRow>[] = [
  { key: 'number', label: 'الوردية', sortable: true },
  { key: 'openedByName', label: 'الكاشير' },
  { key: 'openedAt', label: 'الفتح', sortable: true },
  { key: 'closedAt', label: 'الإغلاق' },
  { key: 'status', label: 'الحالة' },
  { key: 'expectedCash', label: 'المتوقع', numeric: true },
  { key: 'countedCash', label: 'المعدود', numeric: true },
  { key: 'variance', label: 'الفرق', numeric: true },
  { key: 'actions', label: '' },
];

const canManage = computed(() => auth.can('pos', 'write') && (auth.role === 'admin' || auth.role === 'manager'));

async function forceClose(row: ShiftRow) {
  const ok = await confirmDialog({ title: `إغلاق إجباري للوردية ${row.number}؟`, message: 'سيتم إغلاقها بافتراض عدم وجود فرق ما لم تُدخل مبلغاً معدوداً.', confirmText: 'إغلاق' });
  if (!ok) return;
  try {
    await forceClosePosShift(row.id);
    toast.success('تم إغلاق الوردية');
    reload();
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="إدارة الورديات" subtitle="الورديات المفتوحة والمغلقة مع الفروقات" />
    <DataTable :columns="columns" :rows="rows" :loading="loading" :error="error" @retry="reload">
      <template #cell-number="{ row }"><span class="num font-medium">{{ row.number }}</span></template>
      <template #cell-openedAt="{ row }"><span class="num text-text-secondary">{{ formatDateTime(row.openedAt) }}</span></template>
      <template #cell-closedAt="{ row }"><span class="num text-text-secondary">{{ row.closedAt ? formatDateTime(row.closedAt) : '—' }}</span></template>
      <template #cell-status="{ row }"><StatusBadge :tone="row.status === 'OPEN' ? 'primary' : 'neutral'" :label="row.status === 'OPEN' ? 'مفتوحة' : 'مغلقة'" /></template>
      <template #cell-expectedCash="{ row }"><MoneyText :value="row.expectedCash ?? row.expectedCash" dash-zero /></template>
      <template #cell-countedCash="{ row }"><MoneyText :value="row.countedCash" dash-zero /></template>
      <template #cell-variance="{ row }">
        <MoneyText v-if="row.variance !== undefined" :value="row.variance" signed />
        <span v-else class="text-text-secondary">—</span>
      </template>
      <template #cell-actions="{ row }">
        <AppButton v-if="row.status === 'OPEN' && canManage" size="sm" variant="ghost" :icon="RotateCcw" @click="forceClose(row)">إغلاق إجباري</AppButton>
        <AppButton v-if="row.status === 'CLOSED'" size="sm" variant="ghost" :icon="Printer" :to="{ name: 'pos-shift-report', params: { id: row.id } }">تقرير Z</AppButton>
      </template>
    </DataTable>
  </div>
</template>

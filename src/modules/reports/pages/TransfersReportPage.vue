<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Transfers: in transit, received with shortages"). */
import { computed, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getTransfersReport } from '../services/reportService';
import type { TransferReportRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getTransfersReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const STATUS_LABEL: Record<string, string> = { DRAFT: 'مسودة', SENT: 'تم الإرسال', RECEIVED: 'تم الاستلام', REJECTED: 'مرفوض' };
const STATUS_TONE: Record<string, 'neutral' | 'warning' | 'success' | 'danger'> = { DRAFT: 'neutral', SENT: 'warning', RECEIVED: 'success', REJECTED: 'danger' };

const insights = computed(() => {
  if (!data.value?.length) return null;
  const shortages = data.value.filter((r) => r.shortageQty > 0.001);
  const inTransit = data.value.filter((r) => r.status === 'SENT').length;
  return { headline: `${formatNumber(data.value.length)} تحويلاً خلال الفترة`, metrics: [{ label: 'قيد النقل', value: String(inTransit) }, { label: 'تحويلات بها عجز', value: String(shortages.length) }] };
});

const columns: Column<TransferReportRow>[] = [
  { key: 'number', label: 'رقم التحويل', sortable: true },
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'fromBranch', label: 'من فرع', sortable: true },
  { key: 'toBranch', label: 'إلى فرع', sortable: true },
  { key: 'status', label: 'الحالة' },
  { key: 'sentQty', label: 'المُرسل', numeric: true, sortable: true },
  { key: 'receivedQty', label: 'المُستلم', numeric: true, sortable: true },
  { key: 'shortageQty', label: 'العجز', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'تقرير التحويلات بين الفروع',
    columns: columns.map((c) => c.label),
    rows: data.value.map((r) => [r.number, r.date, r.fromBranch, r.toBranch, STATUS_LABEL[r.status] ?? r.status, r.sentQty, r.receivedQty, r.shortageQty]),
  },
);
</script>

<template>
  <ReportShell title="تقرير التحويلات بين الفروع" subtitle="حالة كل تحويل وأي عجز عند الاستلام" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <DataTable :columns="columns" :rows="data" :page-size="0" clickable @row-click="() => $router.push('/inventory/transfers')">
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
      <template #cell-status="{ row }"><StatusBadge :tone="STATUS_TONE[row.status] ?? 'neutral'" :label="STATUS_LABEL[row.status] ?? row.status" /></template>
      <template #cell-shortageQty="{ row }"><span class="num" :class="row.shortageQty > 0 ? 'text-danger' : ''">{{ formatNumber(row.shortageQty) }}</span></template>
    </DataTable>
  </ReportShell>
</template>

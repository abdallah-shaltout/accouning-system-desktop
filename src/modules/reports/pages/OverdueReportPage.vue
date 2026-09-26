<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Overdue invoices: with a contact action (WhatsApp link), sortable by days overdue"). */
import { computed, ref, watch } from 'vue';
import { MessageCircle } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getOverdueReport } from '../services/reportService';
import type { OverdueRow } from '../types';

const kind = ref<'customer' | 'supplier'>('customer');
const { data, loading, error, reload } = useAsync(() => getOverdueReport(kind.value), { immediate: false });
watch(kind, reload, { immediate: true });

const totalOutstanding = computed(() => (data.value ?? []).reduce((a, r) => a + r.outstanding, 0));
const insights = computed(() => {
  if (!data.value?.length) return null;
  const over90 = data.value.filter((r) => r.daysOverdue > 90).length;
  return { headline: `${formatNumber(data.value.length)} مستند متأخر بإجمالي ${formatNumber(totalOutstanding.value)}`, metrics: [{ label: 'متأخر أكثر من 90 يوماً', value: String(over90) }] };
});

function waLink(phone?: string): string | null {
  if (!phone) return null;
  const digits = phone.replace(/[^\d]/g, '');
  return `https://wa.me/${digits}`;
}

const columns: Column<OverdueRow>[] = [
  { key: 'number', label: 'المستند', sortable: true },
  { key: 'partyName', label: 'الجهة', sortable: true },
  { key: 'dueDate', label: 'تاريخ الاستحقاق', sortable: true },
  { key: 'daysOverdue', label: 'أيام التأخر', numeric: true, sortable: true },
  { key: 'outstanding', label: 'المتبقي', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'المستندات المتأخرة',
    columns: columns.map((c) => c.label),
    rows: data.value.map((r) => [r.number, r.partyName, r.dueDate ? formatDate(r.dueDate) : '', r.daysOverdue, r.outstanding]),
  },
);
</script>

<template>
  <ReportShell title="المستندات المتأخرة" subtitle="فواتير وأوامر شراء تجاوزت تاريخ الاستحقاق" :loading="loading && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <SegmentedControl v-model="kind" :options="[{ value: 'customer', label: 'عملاء' }, { value: 'supplier', label: 'موردون' }]" />
    </template>

    <DataTable :columns="columns" :rows="data" :page-size="0" clickable @row-click="(r) => $router.push(r.kind === 'invoice' ? { name: 'invoice', params: { id: r.id } } : { name: 'purchase', params: { id: r.id } })">
      <template #cell-partyName="{ row }">
        <div class="flex items-center gap-2">
          <span>{{ row.partyName }}</span>
          <a v-if="waLink(row.phone)" :href="waLink(row.phone)!" target="_blank" rel="noopener" class="no-print text-success hover:text-success/80" title="تواصل عبر واتساب" @click.stop>
            <MessageCircle class="size-3.5" />
          </a>
        </div>
      </template>
      <template #cell-dueDate="{ row }"><span class="num text-text-secondary">{{ row.dueDate ? formatDate(row.dueDate) : '—' }}</span></template>
      <template #cell-daysOverdue="{ row }"><StatusBadge :tone="row.daysOverdue > 90 ? 'danger' : row.daysOverdue > 30 ? 'warning' : 'neutral'" :label="`${row.daysOverdue} يوم`" /></template>
      <template #cell-outstanding="{ row }"><MoneyText :value="row.outstanding" plain class="font-medium" /></template>
    </DataTable>
  </ReportShell>
</template>

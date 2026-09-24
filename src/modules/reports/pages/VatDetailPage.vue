<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "VAT detail: line-level listing per document with category, net, VAT — the audit trail behind each box"). */
import { computed, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDate, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getVatDetail } from '../services/reportService';
import type { VatDetailRow } from '../types';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getVatDetail({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const CATEGORY_LABEL: Record<string, string> = { S: 'أساسية', Z: 'صفرية', E: 'معفاة', O: 'خارج النطاق' };

const insights = computed(() => {
  if (!data.value?.length) return null;
  return { headline: `${formatNumber(data.value.length)} سطراً في السجل التفصيلي للضريبة`, metrics: [{ label: 'إجمالي الضريبة', value: formatNumber(data.value.reduce((a, r) => a + r.vat, 0)) }] };
});

const columns: Column<VatDetailRow>[] = [
  { key: 'date', label: 'التاريخ', sortable: true },
  { key: 'documentNumber', label: 'المستند', sortable: true },
  { key: 'productName', label: 'الصنف / البيان', sortable: true },
  { key: 'category', label: 'الفئة' },
  { key: 'net', label: 'الصافي', numeric: true, sortable: true },
  { key: 'vat', label: 'الضريبة', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'التفصيل الضريبي',
    columns: columns.map((c) => c.label),
    rows: data.value.map((r) => [formatDate(r.date), r.documentNumber, r.productName, CATEGORY_LABEL[r.category] ?? r.category, r.net, r.vat]),
  },
);
</script>

<template>
  <ReportShell title="التفصيل الضريبي" subtitle="السجل التفصيلي وراء كل مربع في ملخص الضريبة" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <DataTable :columns="columns" :rows="data" :page-size="0">
      <template #cell-date="{ row }"><span class="num text-text-secondary">{{ formatDate(row.date) }}</span></template>
      <template #cell-category="{ row }"><span class="text-xs text-text-secondary">{{ CATEGORY_LABEL[row.category] ?? row.category }} ({{ row.rate }}%)</span></template>
      <template #cell-net="{ row }"><MoneyText :value="row.net" plain signed /></template>
      <template #cell-vat="{ row }"><MoneyText :value="row.vat" plain signed /></template>
    </DataTable>
  </ReportShell>
</template>

<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §2 "Day book (دفتر اليومية): printable journal for a period").
 * Phase 2 left a browser-print day-book on the journal list page — this is the proper reports-hub
 * version: same underlying data (every posted entry in the period, with its lines), but as a normal
 * report with the shared filter bar / drill-down / exports instead of a standalone print route.
 */
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getDayBook } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getDayBook({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const totals = computed(() => ({
  debit: (data.value ?? []).reduce((a, e) => a + e.lines.reduce((s, l) => s + l.debit, 0), 0),
  credit: (data.value ?? []).reduce((a, e) => a + e.lines.reduce((s, l) => s + l.credit, 0), 0),
}));

const insights = computed(() => {
  if (!data.value?.length) return null;
  return { headline: `${formatNumber(data.value.length)} قيد خلال الفترة`, metrics: [{ label: 'إجمالي الحركة', value: formatNumber(totals.value.debit) }] };
});

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  const rows: (string | number)[][] = [];
  for (const e of d) {
    rows.push([e.number, formatDateTime(e.date), e.description, '', '']);
    for (const l of e.lines) rows.push(['', '', `${l.accountCode} ${l.accountName}`, l.debit, l.credit]);
  }
  return { title: 'دفتر اليومية', columns: ['رقم القيد', 'التاريخ', 'البيان / الحساب', 'مدين', 'دائن'], rows };
});
</script>

<template>
  <ReportShell title="دفتر اليومية" subtitle="كل القيود المرحّلة خلال الفترة بتفاصيلها" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div class="space-y-3">
      <div v-for="e in data" :key="e.id" class="overflow-hidden rounded-xl border border-border">
        <RouterLink :to="`/accounting/journal/${e.id}`" class="flex items-center justify-between border-b border-border bg-surface px-4 py-2 text-body hover:bg-surface-hover">
          <span class="flex items-center gap-2"><span class="num font-medium text-primary">{{ e.number }}</span><span class="text-text-secondary">{{ e.description }}</span></span>
          <span class="num text-xs text-text-secondary">{{ formatDateTime(e.date) }}</span>
        </RouterLink>
        <table class="w-full text-body">
          <tbody>
            <tr v-for="(l, i) in e.lines" :key="i" class="border-b border-border last:border-0">
              <td class="px-4 py-1.5"><span class="num text-xs text-text-secondary">{{ l.accountCode }}</span> {{ l.accountName }}</td>
              <td class="w-32 px-3 py-1.5"><MoneyText :value="l.debit" plain dash-zero /></td>
              <td class="w-32 px-3 py-1.5"><MoneyText :value="l.credit" plain dash-zero /></td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-if="data && !data.length" class="py-8 text-center text-xs text-text-secondary">لا توجد قيود في هذه الفترة</p>
      <div v-if="data?.length" class="flex items-center justify-between rounded-xl border-2 border-border px-4 py-3 text-ui font-semibold">
        <span>الإجمالي</span>
        <span class="flex gap-6"><MoneyText :value="totals.debit" /> <MoneyText :value="totals.credit" /></span>
      </div>
    </div>
  </ReportShell>
</template>

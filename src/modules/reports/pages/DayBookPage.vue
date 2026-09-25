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
import { formatDate, formatDateTime, formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { count, kpis, money, row, table as printTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
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

// Official print: every entry as a header band (number · date · description) over its lines.
const print = computed<ReportPrintSpec | null>(() => {
  const d = data.value;
  if (!d) return null;
  const m = (v: number) => money(v, { dashZero: true });
  return {
    signatures: true,
    blocks: [
      kpis([
        { label: 'عدد القيود', value: count(d.length) },
        { label: 'إجمالي المدين', value: money(totals.value.debit), emphasis: true },
        { label: 'إجمالي الدائن', value: money(totals.value.credit), emphasis: true },
      ]),
      printTable(
        [
          { label: 'الرمز', dim: true, width: 0.8 },
          { label: 'الحساب', width: 4 },
          { label: 'مدين', numeric: true, width: 1.3 },
          { label: 'دائن', numeric: true, width: 1.3 },
        ],
        [
          ...d.flatMap((e) => [
            // Arabic words between the Latin number and the date keep bidi from fusing them into one LTR run.
            row([`قيد ${e.number} بتاريخ ${formatDate(e.date)} — ${e.description}`], 'subhead'),
            ...e.lines.map((l) => row([l.accountCode, l.accountName, m(l.debit), m(l.credit)])),
          ]),
          row(['', 'الإجمالي', money(totals.value.debit), money(totals.value.credit)], 'total'),
        ],
        'لا توجد قيود في هذه الفترة',
      ),
    ],
  };
});
</script>

<template>
  <ReportShell title="دفتر اليومية" subtitle="كل القيود المرحّلة خلال الفترة بتفاصيلها" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :print="print" :insights="insights" @retry="reload">
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

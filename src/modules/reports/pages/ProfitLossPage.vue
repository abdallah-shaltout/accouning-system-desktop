<script setup lang="ts">
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import StatementSection from '../components/StatementSection.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getProfitAndLoss } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getProfitAndLoss({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const margin = computed(() => (data.value && data.value.netRevenue ? (data.value.netIncome / data.value.netRevenue) * 100 : 0));
const grossMargin = computed(() => (data.value && data.value.netRevenue ? (data.value.grossProfit / data.value.netRevenue) * 100 : 0));

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  const rows: (string | number)[][] = [];
  const section = (title: string, lines: typeof d.revenue, total: number) => {
    rows.push([title, '', '']);
    for (const l of lines) rows.push(['', `${l.code} ${l.name}`, l.amount]);
    rows.push(['', `إجمالي ${title}`, total]);
  };
  section('الإيرادات', d.revenue, d.netRevenue);
  section('تكلفة المبيعات', d.cogs, d.totalCogs);
  rows.push(['مجمل الربح', '', d.grossProfit]);
  section('المصروفات', d.expenses, d.totalExpenses);
  rows.push(['صافي الربح', '', d.netIncome]);
  return { title: 'قائمة الدخل', columns: ['البند', 'الحساب', 'المبلغ'], rows };
});
</script>

<template>
  <ReportShell title="قائمة الدخل" subtitle="الأرباح والخسائر للفترة المحددة" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="grid items-start gap-5 xl:grid-cols-[1fr_300px]">
      <div class="space-y-4">
        <StatementSection title="الإيرادات" :lines="data.revenue" :total="data.netRevenue" total-label="صافي الإيرادات" />
        <StatementSection title="تكلفة المبيعات" :lines="data.cogs" :total="data.totalCogs" />
        <div class="flex items-center justify-between rounded-xl border border-border px-4 py-3 text-ui font-semibold">
          <span>مجمل الربح</span>
          <MoneyText :value="data.grossProfit" signed />
        </div>
        <StatementSection title="المصروفات" :lines="data.expenses" :total="data.totalExpenses" />
        <div class="flex items-center justify-between rounded-xl border-2 px-4 py-3.5 text-lead font-semibold" :class="data.netIncome >= 0 ? 'border-success/50' : 'border-danger/50'">
          <span>{{ data.netIncome >= 0 ? 'صافي الربح' : 'صافي الخسارة' }}</span>
          <MoneyText :value="data.netIncome" signed />
        </div>
      </div>
      <div class="no-print space-y-3">
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">هامش الربح الإجمالي</p>
          <p class="num mt-1 text-xl font-semibold">{{ formatNumber(grossMargin, 1) }}%</p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">هامش صافي الربح</p>
          <p class="num mt-1 text-xl font-semibold" :class="margin < 0 ? 'text-danger' : ''">{{ formatNumber(margin, 1) }}%</p>
        </div>
        <p class="text-tiny leading-5 text-text-secondary">اضغط على أي حساب لعرض كشف الحساب المفصل.</p>
      </div>
    </div>
  </ReportShell>
</template>

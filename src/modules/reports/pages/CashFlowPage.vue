<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Cash-flow statement" — new, indirect method). */
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getCashFlowStatement } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getCashFlowStatement({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  return {
    headline: d.netChange >= 0 ? `زيادة صافية في النقدية قدرها ${formatNumber(d.netChange)}` : `نقص صافٍ في النقدية قدره ${formatNumber(Math.abs(d.netChange))}`,
    metrics: [
      { label: 'النقدية من التشغيل', value: formatNumber(d.operatingCash) },
      { label: 'رصيد النقدية الختامي', value: formatNumber(d.closingCash) },
    ],
  };
});

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  const rows: (string | number)[][] = [
    ['صافي الربح', d.netIncome],
    ...d.operatingAdjustments.map((l) => [l.label, l.amount]),
    ['صافي النقدية من التشغيل', d.operatingCash],
    ...d.investing.map((l) => [l.label, l.amount]),
    ['صافي النقدية من الاستثمار', d.investingCash],
    ...d.financing.map((l) => [l.label, l.amount]),
    ['صافي النقدية من التمويل', d.financingCash],
    ['صافي التغير في النقدية', d.netChange],
    ['رصيد النقدية أول المدة', d.openingCash],
    ['رصيد النقدية آخر المدة', d.closingCash],
  ];
  return { title: 'قائمة التدفقات النقدية', columns: ['البند', 'المبلغ'], rows };
});
</script>

<template>
  <ReportShell
    title="قائمة التدفقات النقدية"
    subtitle="الطريقة غير المباشرة — من صافي الربح إلى التغير في النقدية"
    :from="from"
    :to="to"
    :loading="(loading || !ready) && !data"
    :error="error"
    :table="table"
    :insights="insights"
    @retry="reload"
  >
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="grid gap-5 lg:grid-cols-3">
      <section class="overflow-hidden rounded-xl border border-border lg:col-span-2">
        <h3 class="border-b border-border bg-surface px-4 py-2 text-xs font-medium text-text-secondary">الأنشطة التشغيلية</h3>
        <ul class="divide-y divide-border text-body">
          <li class="flex items-center justify-between px-4 py-2"><span>صافي الربح</span><MoneyText :value="data.netIncome" signed /></li>
          <li v-for="l in data.operatingAdjustments" :key="l.label" class="flex items-center justify-between px-4 py-2 text-text-secondary">
            <span>{{ l.label }}</span><MoneyText :value="l.amount" plain signed />
          </li>
        </ul>
        <div class="flex items-center justify-between border-t border-border bg-surface px-4 py-2.5 font-semibold"><span>صافي النقدية من التشغيل</span><MoneyText :value="data.operatingCash" signed /></div>

        <h3 class="border-y border-border bg-surface px-4 py-2 text-xs font-medium text-text-secondary">الأنشطة الاستثمارية</h3>
        <ul class="divide-y divide-border text-body">
          <li v-for="l in data.investing" :key="l.label" class="flex items-center justify-between px-4 py-2 text-text-secondary"><span>{{ l.label }}</span><MoneyText :value="l.amount" plain signed /></li>
          <li v-if="!data.investing.length" class="px-4 py-2 text-xs text-text-secondary">لا توجد حركات</li>
        </ul>
        <div class="flex items-center justify-between border-t border-border bg-surface px-4 py-2.5 font-semibold"><span>صافي النقدية من الاستثمار</span><MoneyText :value="data.investingCash" signed /></div>

        <h3 class="border-y border-border bg-surface px-4 py-2 text-xs font-medium text-text-secondary">الأنشطة التمويلية</h3>
        <ul class="divide-y divide-border text-body">
          <li v-for="l in data.financing" :key="l.label" class="flex items-center justify-between px-4 py-2 text-text-secondary"><span>{{ l.label }}</span><MoneyText :value="l.amount" plain signed /></li>
          <li v-if="!data.financing.length" class="px-4 py-2 text-xs text-text-secondary">لا توجد حركات</li>
        </ul>
        <div class="flex items-center justify-between border-t border-border bg-surface px-4 py-2.5 font-semibold"><span>صافي النقدية من التمويل</span><MoneyText :value="data.financingCash" signed /></div>
      </section>

      <div class="space-y-3">
        <div class="rounded-xl border-2 px-4 py-3.5" :class="data.netChange >= 0 ? 'border-success/50' : 'border-danger/50'">
          <p class="text-xs text-text-secondary">صافي التغير في النقدية</p>
          <p class="num mt-1 text-xl font-semibold"><MoneyText :value="data.netChange" signed /></p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">رصيد النقدية أول المدة</p>
          <p class="mt-1 text-lg font-semibold"><MoneyText :value="data.openingCash" /></p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">رصيد النقدية آخر المدة</p>
          <p class="mt-1 text-lg font-semibold"><MoneyText :value="data.closingCash" /></p>
        </div>
      </div>
    </div>
  </ReportShell>
</template>

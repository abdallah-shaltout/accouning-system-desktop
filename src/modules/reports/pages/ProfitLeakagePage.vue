<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Profit leakage: discounts + price overrides + returns + shrinkage + write-offs as % of sales"). */
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getProfitLeakageReport } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getProfitLeakageReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const rows = computed(() =>
  data.value
    ? [
        { label: 'الخصومات', value: data.value.discounts },
        { label: 'المرتجعات', value: data.value.returns },
        { label: 'الهالك / التلف', value: data.value.writeOffs },
        { label: 'عجز الجرد', value: data.value.shrinkage },
      ]
    : [],
);

const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  return { headline: `${formatNumber(d.leakagePct, 1)}% من المبيعات فُقدت خلال الفترة`, metrics: [{ label: 'إجمالي الفاقد', value: formatNumber(d.totalLeakage) }] };
});

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'تقرير تسرب الربح', columns: ['البند', 'القيمة'], rows: [...rows.value.map((r) => [r.label, r.value]), ['الإجمالي', data.value.totalLeakage], ['النسبة من صافي المبيعات', `${data.value.leakagePct}%`]] },
);
</script>

<template>
  <ReportShell title="تقرير تسرب الربح" subtitle="الخصومات والمرتجعات والهالك وعجز الجرد كنسبة من المبيعات" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="grid gap-5 lg:grid-cols-[1fr_260px]">
      <div class="overflow-hidden rounded-xl border border-border">
        <table class="w-full text-body">
          <tbody>
            <tr v-for="r in rows" :key="r.label" class="border-b border-border last:border-0">
              <td class="px-4 py-2.5">{{ r.label }}</td>
              <td class="w-40 px-4 py-2.5"><MoneyText :value="r.value" plain /></td>
            </tr>
          </tbody>
          <tfoot class="border-t-2 border-border bg-surface font-semibold">
            <tr>
              <td class="px-4 py-2.5">إجمالي الفاقد</td>
              <td class="px-4 py-2.5"><MoneyText :value="data.totalLeakage" /></td>
            </tr>
          </tfoot>
        </table>
      </div>
      <div class="space-y-3">
        <div class="rounded-xl border-2 border-warning/50 p-4">
          <p class="text-xs text-text-secondary">النسبة من صافي المبيعات</p>
          <p class="num mt-1 text-2xl font-semibold">{{ formatNumber(data.leakagePct, 1) }}%</p>
        </div>
        <div class="rounded-xl border border-border bg-surface p-4">
          <p class="text-xs text-text-secondary">صافي المبيعات</p>
          <p class="mt-1 text-lg font-semibold"><MoneyText :value="data.netSales" /></p>
        </div>
      </div>
    </div>
  </ReportShell>
</template>

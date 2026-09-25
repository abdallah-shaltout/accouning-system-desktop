<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §3 "P&L by cost center: a column per
 * center, with a drill-down"). Drill-down = clicking a center's column header filters the flat P&L
 * page to that one cost center (query param) rather than duplicating the full statement layout here.
 */
import { computed, ref, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { money, row, table as printTable } from '../print/build';
import type { ReportPrintSpec } from '../print/types';
import { getCostCenterProfitAndLoss } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getCostCenterProfitAndLoss({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const ROWS: { key: 'netRevenue' | 'totalCogs' | 'grossProfit' | 'totalExpenses' | 'netIncome'; label: string; emphasis?: boolean }[] = [
  { key: 'netRevenue', label: 'صافي الإيرادات' },
  { key: 'totalCogs', label: 'تكلفة المبيعات' },
  { key: 'grossProfit', label: 'مجمل الربح', emphasis: true },
  { key: 'totalExpenses', label: 'المصروفات' },
  { key: 'netIncome', label: 'صافي الربح', emphasis: true },
];

const columns = computed(() => {
  if (!data.value) return [];
  const cols = [...data.value.centers];
  if (Math.abs(data.value.unassigned.netRevenue) + Math.abs(data.value.unassigned.totalExpenses) > 0.01) cols.push(data.value.unassigned);
  return cols;
});

const drillDown = ref<string | null>(null);

const table = computed<ExportTable | undefined>(() => {
  if (!data.value) return undefined;
  const rows: (string | number)[][] = ROWS.map((r) => [r.label, ...columns.value.map((c) => c[r.key]), data.value!.total[r.key]]);
  return { title: 'الأرباح والخسائر حسب مركز التكلفة', columns: ['البند', ...columns.value.map((c) => c.name), 'الإجمالي'], rows };
});

const print = computed<ReportPrintSpec | null>(() => {
  const d = data.value;
  if (!d) return null;
  const kind = (key: (typeof ROWS)[number]['key']) => (key === 'netIncome' ? 'grand' : key === 'grossProfit' ? 'subtotal' : 'normal');
  return {
    signatures: true,
    blocks: [
      printTable(
        [{ label: 'البند', width: 1.8 }, ...columns.value.map((c) => ({ label: c.name, numeric: true, width: 1.2 })), { label: 'الإجمالي', numeric: true, width: 1.3 }],
        ROWS.map((r) => row([r.label, ...columns.value.map((c) => money(c[r.key])), money(d.total[r.key])], kind(r.key))),
        'لا توجد مراكز تكلفة ذات حركة في هذه الفترة',
      ),
    ],
  };
});
</script>

<template>
  <ReportShell
    title="الأرباح والخسائر حسب مركز التكلفة"
    subtitle="عمود لكل مركز تكلفة نشط خلال الفترة — اضغط اسم المركز لعرض تفاصيله"
    :from="from"
    :to="to"
    :loading="(loading || !ready) && !data"
    :error="error"
    :table="table"
    :print="print"
    @retry="reload"
  >
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="overflow-x-auto rounded-xl border border-border">
      <table class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border bg-surface">
            <th class="sticky start-0 bg-surface px-4 py-2.5 text-start font-medium">البند</th>
            <th v-for="c in columns" :key="c.costCenterId" class="min-w-32 px-3 py-2.5 text-start font-medium">
              <button type="button" class="hover:text-primary hover:underline" @click="drillDown = c.costCenterId">{{ c.name }}</button>
            </th>
            <th class="min-w-32 bg-surface-hover px-3 py-2.5 text-start font-semibold">الإجمالي</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in ROWS" :key="r.key" class="border-b border-border last:border-0" :class="r.emphasis ? 'bg-surface/60 font-semibold' : ''">
            <td class="sticky start-0 bg-background px-4 py-2">{{ r.label }}</td>
            <td v-for="c in columns" :key="c.costCenterId" class="px-3 py-2"><MoneyText :value="c[r.key]" signed /></td>
            <td class="bg-surface-hover px-3 py-2"><MoneyText :value="data.total[r.key]" signed /></td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="drillDown && data" class="mt-4 rounded-xl border border-border bg-surface p-4">
      <p class="text-body font-medium">{{ columns.find((c) => c.costCenterId === drillDown)?.name }}</p>
      <dl class="mt-2 grid grid-cols-2 gap-2 text-tiny sm:grid-cols-5">
        <div v-for="r in ROWS" :key="r.key">
          <dt class="text-text-secondary">{{ r.label }}</dt>
          <dd class="num font-medium"><MoneyText :value="columns.find((c) => c.costCenterId === drillDown)![r.key]" signed /></dd>
        </div>
      </dl>
    </div>
  </ReportShell>
</template>

<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Expenses: by category/branch/cost center/month, with the trend"). */
import { computed, watch } from 'vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getExpensesReport } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getExpensesReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  const d = data.value;
  if (!d?.byCategory.length) return null;
  return { headline: `إجمالي المصروفات ${formatNumber(d.total)}`, metrics: [{ label: 'أعلى فئة', value: d.byCategory[0]?.name ?? '' }] };
});

const categoryColumns: Column<{ categoryId: string; name: string; amount: number }>[] = [
  { key: 'name', label: 'الفئة', sortable: true },
  { key: 'amount', label: 'المبلغ', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'تقرير المصروفات', columns: ['الفئة', 'المبلغ'], rows: [...data.value.byCategory.map((r) => [r.name, r.amount]), ['الإجمالي', data.value.total]] },
);
</script>

<template>
  <ReportShell title="تقرير المصروفات" subtitle="المصروفات حسب الفئة والشهر" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="mb-5 rounded-xl border-2 border-border px-4 py-3.5">
      <p class="text-xs text-text-secondary">إجمالي المصروفات</p>
      <p class="mt-1 text-xl font-semibold"><MoneyText :value="data.total" /></p>
    </div>

    <div v-if="data?.byMonth.length" class="no-print mb-5 overflow-hidden rounded-xl border border-border">
      <div class="border-b border-border bg-surface px-4 py-2 text-xs font-medium text-text-secondary">الاتجاه الشهري</div>
      <div class="flex items-end gap-2 overflow-x-auto p-4" style="height: 120px">
        <div v-for="m in data.byMonth" :key="m.month" class="flex min-w-10 flex-col items-center gap-1">
          <div class="w-6 rounded-t bg-primary/70" :style="{ height: `${Math.max(4, (m.amount / Math.max(...data!.byMonth.map((x) => x.amount), 1)) * 90)}px` }" />
          <span class="text-tiny text-text-secondary">{{ m.month.slice(5) }}</span>
        </div>
      </div>
    </div>

    <DataTable :columns="categoryColumns" :rows="data?.byCategory" row-key="categoryId" :page-size="0">
      <template #cell-amount="{ row }"><MoneyText :value="row.amount" plain /></template>
    </DataTable>
  </ReportShell>
</template>

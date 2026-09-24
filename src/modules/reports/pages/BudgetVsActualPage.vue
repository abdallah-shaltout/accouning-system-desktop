<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §2 "Budget vs actual: per cost center") — confirms/upgrades
 * Phase 9's `getCostCenterBudgetVsActual` (docs/v2/10-branches-currencies-cost-centers.md §3), which
 * shipped a working backend query but no reports-hub page yet. This is that page: a fiscal-year
 * picker + bars with variance %, matching the doc's "bars with variance %; an insight when a center
 * passes 90% of its budget" (`nearBudget`).
 */
import { computed, onMounted, ref, watch } from 'vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import { getFiscalYears } from '@/modules/accounting/services/accountingService';
import type { FiscalYear } from '@/modules/accounting/types';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getCostCenterBudgetVsActual } from '../services/reportService';
import type { CostCenterBudgetRow } from '../types';

const fiscalYears = ref<FiscalYear[]>([]);
const fiscalYearId = ref('');

onMounted(async () => {
  fiscalYears.value = await getFiscalYears();
  fiscalYearId.value = fiscalYears.value.find((f) => !f.isClosed)?.id ?? fiscalYears.value[0]?.id ?? '';
});

const { data, loading, error, reload } = useAsync(() => (fiscalYearId.value ? getCostCenterBudgetVsActual(fiscalYearId.value) : Promise.resolve([])), { immediate: false });
watch(fiscalYearId, () => fiscalYearId.value && reload());

const insights = computed(() => {
  if (!data.value?.length) return null;
  const near = data.value.filter((r) => r.nearBudget);
  return { headline: near.length ? `${near.length} مركز تكلفة تجاوز 90% من الميزانية` : 'كل مراكز التكلفة ضمن الميزانية', metrics: [] };
});

const columns: Column<CostCenterBudgetRow>[] = [
  { key: 'name', label: 'مركز التكلفة', sortable: true },
  { key: 'budget', label: 'الميزانية', numeric: true, sortable: true },
  { key: 'actual', label: 'الفعلي', numeric: true, sortable: true },
  { key: 'variancePct', label: 'الانحراف %', numeric: true, sortable: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'الميزانية مقابل الفعلي', columns: columns.map((c) => c.label), rows: data.value.map((r) => [r.name, r.budget, r.actual, r.variancePct]) },
);

function barPct(r: CostCenterBudgetRow) {
  return r.budget > 0 ? Math.min(100, (r.actual / r.budget) * 100) : 0;
}
</script>

<template>
  <ReportShell title="الميزانية مقابل الفعلي" subtitle="لكل مركز تكلفة له ميزانية في السنة المالية المحددة" :loading="loading && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <AppSelect v-model="fiscalYearId" class="w-48" label="السنة المالية" :options="fiscalYears.map((f) => ({ value: f.id, label: f.name }))" />
    </template>

    <div class="mb-5 space-y-3">
      <div v-for="r in data" :key="r.costCenterId" class="rounded-xl border border-border bg-surface p-3.5">
        <div class="mb-1.5 flex items-center justify-between text-body">
          <span class="font-medium">{{ r.name }}</span>
          <span class="num text-xs text-text-secondary">{{ formatNumber(r.actual) }} / {{ formatNumber(r.budget) }} ({{ formatNumber(r.variancePct, 1) }}%)</span>
        </div>
        <div class="h-2 overflow-hidden rounded-full bg-surface-hover">
          <div class="h-full rounded-full" :class="r.nearBudget ? 'bg-danger' : 'bg-primary'" :style="{ width: `${barPct(r)}%` }" />
        </div>
      </div>
      <p v-if="data && !data.length" class="py-6 text-center text-xs text-text-secondary">لا توجد مراكز تكلفة لها ميزانية في هذه السنة المالية</p>
    </div>

    <DataTable :columns="columns" :rows="data" row-key="costCenterId" :page-size="0">
      <template #cell-budget="{ row }"><MoneyText :value="row.budget" plain /></template>
      <template #cell-actual="{ row }"><MoneyText :value="row.actual" plain /></template>
      <template #cell-variancePct="{ row }"><span class="num" :class="row.nearBudget ? 'text-danger' : ''">{{ formatNumber(row.variancePct, 1) }}%</span></template>
    </DataTable>
  </ReportShell>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue';
import { useRouter } from 'vue-router';
import { CircleAlert, CircleCheck } from '@lucide/vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { round2 } from '@/modules/invoices/helpers/totals';
import DimensionFilters from '../components/DimensionFilters.vue';
import ReportShell from '../components/ReportShell.vue';
import { useReportFilters } from '../controllers/useReportFilters';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getTrialBalance } from '../services/reportService';
import type { TrialBalanceRow } from '../types';

const router = useRouter();
const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { branchId, costCenterId, currency, branches, costCenters, currencies, showBranch, showCostCenter, showCurrency, dimensionQuery, syncDimensionsUrl } = useReportFilters();
const { data, loading, error, reload } = useAsync(
  () => getTrialBalance({ from: from.value || undefined, to: to.value || undefined, ...dimensionQuery.value }),
  { immediate: false },
);
watch([from, to, branchId, costCenterId, currency, ready], () => {
  if (!ready.value) return;
  syncUrl();
  syncDimensionsUrl();
  reload();
});

const totals = computed(() => {
  const rows = data.value ?? [];
  const s = (f: (r: TrialBalanceRow) => number) => round2(rows.reduce((a, r) => a + f(r), 0));
  return {
    opening: s((r) => r.openingBalance),
    debit: s((r) => r.periodDebit),
    credit: s((r) => r.periodCredit),
    closingDebit: s((r) => r.closingDebit),
    closingCredit: s((r) => r.closingCredit),
  };
});
const balanced = computed(() => Math.abs(totals.value.closingDebit - totals.value.closingCredit) < 0.01);

// v2 (docs/v2/13 §1 "insights box"): a plain per-page summary until Phase 10's insight engine merges — TODO(phase 10).
const insights = computed(() => {
  if (!data.value?.length) return null;
  const accountsWithBalance = data.value.filter((r) => r.closingDebit > 0 || r.closingCredit > 0).length;
  return {
    headline: balanced.value ? 'الأرصدة متوازنة — لا يوجد فرق بين إجمالي المدين والدائن' : 'يوجد فرق بين إجمالي المدين والدائن — راجع القيود اليدوية',
    metrics: [
      { label: 'عدد الحسابات ذات الرصيد', value: String(accountsWithBalance) },
      { label: 'إجمالي الأرصدة المدينة', value: totals.value.closingDebit.toLocaleString('ar') },
    ],
  };
});

const columns: Column<TrialBalanceRow>[] = [
  { key: 'code', label: 'الرمز', width: '80px' },
  { key: 'name', label: 'الحساب' },
  { key: 'openingBalance', label: 'رصيد أول المدة', numeric: true },
  { key: 'periodDebit', label: 'حركة مدينة', numeric: true },
  { key: 'periodCredit', label: 'حركة دائنة', numeric: true },
  { key: 'closingDebit', label: 'رصيد مدين', numeric: true },
  { key: 'closingCredit', label: 'رصيد دائن', numeric: true },
];

const table = computed<ExportTable | undefined>(() =>
  data.value && {
    title: 'ميزان المراجعة',
    columns: columns.map((c) => c.label),
    rows: [
      ...data.value.map((r) => [r.code, r.name, r.openingBalance, r.periodDebit, r.periodCredit, r.closingDebit, r.closingCredit]),
      ['', 'الإجمالي', totals.value.opening, totals.value.debit, totals.value.credit, totals.value.closingDebit, totals.value.closingCredit],
    ],
  },
);
</script>

<template>
  <ReportShell
    title="ميزان المراجعة"
    subtitle="أرصدة الحسابات في نهاية الفترة — يجب أن يتساوى المدين والدائن"
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
      <DimensionFilters
        v-model:branch-id="branchId"
        v-model:cost-center-id="costCenterId"
        v-model:currency="currency"
        :show-branch="showBranch"
        :show-cost-center="showCostCenter"
        :show-currency="showCurrency"
        :branches="branches"
        :cost-centers="costCenters"
        :currencies="currencies"
      />
    </template>

    <p class="mb-3 flex items-center gap-1.5 text-body" :class="balanced ? 'text-success' : 'text-danger'">
      <CircleCheck v-if="balanced" class="size-4" />
      <CircleAlert v-else class="size-4" />
      {{ balanced ? 'الميزان متوازن' : 'الميزان غير متوازن!' }}
    </p>

    <DataTable :columns="columns" :rows="data" :page-size="0" clickable @row-click="(r) => router.push(`/reports/ledger?account=${r.accountId}&from=${from}&to=${to}`)">
      <template #cell-code="{ row }"><span class="num text-text-secondary">{{ row.code }}</span></template>
      <template #cell-openingBalance="{ row }"><MoneyText :value="row.openingBalance" plain dash-zero /></template>
      <template #cell-periodDebit="{ row }"><MoneyText :value="row.periodDebit" plain dash-zero /></template>
      <template #cell-periodCredit="{ row }"><MoneyText :value="row.periodCredit" plain dash-zero /></template>
      <template #cell-closingDebit="{ row }"><MoneyText :value="row.closingDebit" plain dash-zero class="font-medium" /></template>
      <template #cell-closingCredit="{ row }"><MoneyText :value="row.closingCredit" plain dash-zero class="font-medium" /></template>
      <template #footer>
        <tr>
          <td class="px-3 py-3" colspan="2">الإجمالي</td>
          <td class="px-3 py-3"><MoneyText :value="totals.opening" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.debit" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.credit" plain /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.closingDebit" /></td>
          <td class="px-3 py-3"><MoneyText :value="totals.closingCredit" /></td>
        </tr>
      </template>
    </DataTable>
  </ReportShell>
</template>

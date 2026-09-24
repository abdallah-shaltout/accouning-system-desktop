<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { CircleAlert, CircleCheck } from '@lucide/vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber, todayKey } from '@/modules/core/helpers/format';
import DimensionFilters from '../components/DimensionFilters.vue';
import ReportShell from '../components/ReportShell.vue';
import StatementSection from '../components/StatementSection.vue';
import { useReportFilters } from '../controllers/useReportFilters';
import type { ExportTable } from '../helpers/export';
import { getBalanceSheet } from '../services/reportService';

const route = useRoute();
const router = useRouter();
const asOf = ref(typeof route.query.asOf === 'string' ? route.query.asOf : todayKey());
const { branchId, costCenterId, currency, branches, costCenters, currencies, showBranch, showCostCenter, showCurrency, dimensionQuery, syncDimensionsUrl } = useReportFilters();
const { data, loading, error, reload } = useAsync(() => getBalanceSheet(asOf.value || todayKey(), dimensionQuery.value));
watch([asOf, branchId, costCenterId, currency], () => {
  router.replace({ query: { ...route.query, asOf: asOf.value } });
  syncDimensionsUrl();
  reload();
});

// v2 (docs/v2/13 §1 "insights box"): the plain per-page summary; `ruleKeys` below (passed to
// ReportShell) additionally surfaces real insight-engine hits relevant to the balance sheet.
const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  return {
    headline: d.balanced ? 'الميزانية متوازنة' : 'الميزانية غير متوازنة — راجع القيود',
    metrics: [
      { label: 'إجمالي الأصول', value: formatNumber(d.totalAssets) },
      { label: 'إجمالي الالتزامات', value: formatNumber(d.totalLiabilities) },
    ],
  };
});

const table = computed<ExportTable | undefined>(() => {
  const d = data.value;
  if (!d) return undefined;
  const rows: (string | number)[][] = [];
  const add = (title: string, lines: typeof d.assets, total: number) => {
    rows.push([title, '', '']);
    for (const l of lines) rows.push(['', `${l.code} ${l.name}`, l.amount]);
    rows.push(['', `إجمالي ${title}`, total]);
  };
  add('الأصول', d.assets, d.totalAssets);
  add('الالتزامات', d.liabilities, d.totalLiabilities);
  add('حقوق الملكية', [...d.equity, { accountId: 'ni', code: '', name: 'صافي ربح الفترة (غير مُقفل)', amount: d.unclosedEarnings }], d.totalEquity);
  rows.push(['الالتزامات + حقوق الملكية', '', d.totalLiabilities + d.totalEquity]);
  return { title: 'الميزانية العمومية', columns: ['القسم', 'الحساب', 'المبلغ'], rows };
});
</script>

<template>
  <ReportShell
    title="الميزانية العمومية"
    subtitle="المركز المالي في تاريخ محدد"
    :as-of="asOf"
    :loading="loading && !data"
    :error="error"
    :table="table"
    :insights="insights"
    :rule-keys="['opening-balance-equity', 'year-end']"
    @retry="reload"
  >
    <template #filters>
      <AppInput v-model="asOf" type="date" label="كما في تاريخ" class="w-44" />
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

    <template v-if="data">
      <p class="mb-3 flex items-center gap-1.5 text-body" :class="data.balanced ? 'text-success' : 'text-danger'">
        <CircleCheck v-if="data.balanced" class="size-4" />
        <CircleAlert v-else class="size-4" />
        {{ data.balanced ? 'الأصول = الالتزامات + حقوق الملكية' : 'الميزانية غير متوازنة!' }}
      </p>
      <div class="grid items-start gap-5 lg:grid-cols-2">
        <StatementSection title="الأصول" :lines="data.assets" :total="data.totalAssets" emphasis />
        <div class="space-y-4">
          <StatementSection title="الالتزامات" :lines="data.liabilities" :total="data.totalLiabilities" />
          <StatementSection title="حقوق الملكية" :lines="data.equity" :total="data.totalEquity">
            <li class="flex items-center justify-between px-4 py-2">
              <span class="flex items-center gap-2"><span class="num text-xs text-text-secondary">3300</span>صافي ربح الفترة (غير مُقفل)</span>
              <MoneyText :value="data.unclosedEarnings" plain :class="data.unclosedEarnings < 0 && 'text-danger'" />
            </li>
          </StatementSection>
          <div class="flex items-center justify-between rounded-xl border-2 border-border px-4 py-3 text-ui font-semibold">
            <span>الالتزامات + حقوق الملكية</span>
            <MoneyText :value="data.totalLiabilities + data.totalEquity" />
          </div>
        </div>
      </div>
    </template>
  </ReportShell>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { CircleAlert, CircleCheck } from '@lucide/vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { todayKey } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import StatementSection from '../components/StatementSection.vue';
import type { ExportTable } from '../helpers/export';
import { getBalanceSheet } from '../services/reportService';

const route = useRoute();
const router = useRouter();
const asOf = ref(typeof route.query.asOf === 'string' ? route.query.asOf : todayKey());
const { data, loading, error, reload } = useAsync(() => getBalanceSheet(asOf.value || todayKey()));
watch(asOf, () => {
  router.replace({ query: { asOf: asOf.value } });
  reload();
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
  <ReportShell title="الميزانية العمومية" subtitle="المركز المالي في تاريخ محدد" :as-of="asOf" :loading="loading && !data" :error="error" :table="table" @retry="reload">
    <template #filters>
      <AppInput v-model="asOf" type="date" label="كما في تاريخ" class="w-44" />
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

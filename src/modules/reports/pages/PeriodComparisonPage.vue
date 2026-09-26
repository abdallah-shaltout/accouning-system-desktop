<script setup lang="ts">
/** v2 phase 12 (docs/v2/13-reports.md §2 "Period comparison: any two periods, key lines side by side with Δ and Δ%"). */
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { daysAgoKey, formatNumber, todayKey } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import type { ExportTable } from '../helpers/export';
import { getPeriodComparison } from '../services/reportService';

const route = useRoute();
const router = useRouter();
const q = route.query;
const aFrom = ref(typeof q.aFrom === 'string' ? q.aFrom : daysAgoKey(29));
const aTo = ref(typeof q.aTo === 'string' ? q.aTo : todayKey());
const bFrom = ref(typeof q.bFrom === 'string' ? q.bFrom : daysAgoKey(59));
const bTo = ref(typeof q.bTo === 'string' ? q.bTo : daysAgoKey(30));

const { data, loading, error, reload } = useAsync(() => getPeriodComparison({ from: aFrom.value, to: aTo.value }, { from: bFrom.value, to: bTo.value }), { immediate: false });
watch([aFrom, aTo, bFrom, bTo], () => {
  router.replace({ query: { aFrom: aFrom.value, aTo: aTo.value, bFrom: bFrom.value, bTo: bTo.value } });
  reload();
}, { immediate: true });

const insights = computed(() => {
  if (!data.value?.length) return null;
  const net = data.value.find((l) => l.label === 'صافي الربح');
  if (!net) return null;
  return { headline: `صافي الربح تغيّر بنسبة ${net.deltaPct >= 0 ? '+' : ''}${formatNumber(net.deltaPct, 1)}% بين الفترتين`, metrics: [] };
});

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'مقارنة الفترات', columns: ['البند', 'الفترة الأولى', 'الفترة الثانية', 'الفرق', 'الفرق %'], rows: data.value.map((l) => [l.label, l.a, l.b, l.delta, l.deltaPct]) },
);
</script>

<template>
  <ReportShell title="مقارنة الفترات" subtitle="أي فترتين جنباً إلى جنب — البنود الرئيسية من قائمة الدخل" :loading="loading && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <div class="flex items-end gap-2">
        <AppDatePicker v-model="aFrom" label="الفترة الأولى من" class="w-40" />
        <AppDatePicker v-model="aTo" label="إلى" class="w-40" />
      </div>
      <div class="flex items-end gap-2">
        <AppDatePicker v-model="bFrom" label="الفترة الثانية من" class="w-40" />
        <AppDatePicker v-model="bTo" label="إلى" class="w-40" />
      </div>
    </template>

    <div v-if="data" class="overflow-hidden rounded-xl border border-border">
      <table class="w-full text-body">
        <thead class="bg-surface text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2.5 text-start font-medium">البند</th>
            <th class="px-3 py-2.5 text-start font-medium">الفترة الأولى</th>
            <th class="px-3 py-2.5 text-start font-medium">الفترة الثانية</th>
            <th class="px-3 py-2.5 text-start font-medium">الفرق</th>
            <th class="px-4 py-2.5 text-start font-medium">الفرق %</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in data" :key="l.label" class="border-b border-border last:border-0">
            <td class="px-4 py-2.5">{{ l.label }}</td>
            <td class="px-3 py-2.5"><MoneyText :value="l.a" plain /></td>
            <td class="px-3 py-2.5"><MoneyText :value="l.b" plain /></td>
            <td class="px-3 py-2.5"><MoneyText :value="l.delta" plain signed /></td>
            <td class="px-4 py-2.5"><span class="num" :class="l.deltaPct < 0 ? 'text-danger' : 'text-success'">{{ l.deltaPct >= 0 ? '+' : '' }}{{ formatNumber(l.deltaPct, 1) }}%</span></td>
          </tr>
        </tbody>
      </table>
    </div>
  </ReportShell>
</template>

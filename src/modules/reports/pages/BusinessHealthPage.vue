<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §2 "Business health: liquidity (current ratio), profitability
 * (net margin), debt, collection (DSO), each scored 0–25, with an explanation in plain Arabic").
 */
import { computed, watch } from 'vue';
import DateRangeFilter from '@/modules/core/components/ui/DateRangeFilter.vue';
import { useAsync } from '@/modules/core/controllers/useAsync';
import { formatNumber } from '@/modules/core/helpers/format';
import ReportShell from '../components/ReportShell.vue';
import { useReportRange } from '../controllers/useReportRange';
import type { ExportTable } from '../helpers/export';
import { getBusinessHealthReport } from '../services/reportService';

const { from, to, fiscalStart, ready, syncUrl } = useReportRange();
const { data, loading, error, reload } = useAsync(() => getBusinessHealthReport({ from: from.value || undefined, to: to.value || undefined }), { immediate: false });
watch([from, to, ready], () => {
  if (!ready.value) return;
  syncUrl();
  reload();
});

const insights = computed(() => {
  const d = data.value;
  if (!d) return null;
  const label = d.total >= 80 ? 'ممتازة' : d.total >= 60 ? 'جيدة' : d.total >= 40 ? 'تحتاج متابعة' : 'ضعيفة';
  return { headline: `الصحة المالية العامة: ${formatNumber(d.total)} من 100 — ${label}`, metrics: [] };
});

const table = computed<ExportTable | undefined>(() =>
  data.value && { title: 'الصحة المالية للمنشأة', columns: ['المحور', 'النتيجة (من 25)', 'الشرح'], rows: [...data.value.scores.map((s) => [s.label, s.score, s.explanation]), ['الإجمالي', data.value.total, '']] },
);

function scoreColor(score: number) {
  if (score >= 18) return 'text-success';
  if (score >= 12) return 'text-warning';
  return 'text-danger';
}
</script>

<template>
  <ReportShell title="الصحة المالية للمنشأة" subtitle="مؤشر مركّب من السيولة والربحية والمديونية والتحصيل" :from="from" :to="to" :loading="(loading || !ready) && !data" :error="error" :table="table" :insights="insights" @retry="reload">
    <template #filters>
      <DateRangeFilter v-model:from="from" v-model:to="to" :fiscal-start="fiscalStart" />
    </template>

    <div v-if="data" class="space-y-5">
      <div class="rounded-xl border-2 border-primary/40 bg-primary/5 p-5 text-center">
        <p class="text-xs text-text-secondary">النتيجة الإجمالية</p>
        <p class="num mt-1 text-4xl font-bold">{{ formatNumber(data.total) }}<span class="text-lg text-text-secondary"> / 100</span></p>
      </div>

      <div class="grid gap-3 sm:grid-cols-2">
        <div v-for="s in data.scores" :key="s.key" class="rounded-xl border border-border bg-surface p-4">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-body font-medium">{{ s.label }}</span>
            <span class="num text-lg font-semibold" :class="scoreColor(s.score)">{{ formatNumber(s.score) }} / 25</span>
          </div>
          <div class="mb-2 h-2 overflow-hidden rounded-full bg-surface-hover">
            <div class="h-full rounded-full bg-current" :class="scoreColor(s.score)" :style="{ width: `${(s.score / 25) * 100}%` }" />
          </div>
          <p class="text-tiny leading-5 text-text-secondary">{{ s.explanation }}</p>
        </div>
      </div>
    </div>
  </ReportShell>
</template>

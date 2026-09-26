<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Gauge } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { computePerfStats, loadChannel, slowestLongTasks } from '../../services/diagnosticsReadService';
import type { LogEntry, PerfStat } from '../../types';

/** p50/p95 per service/route + slowest long tasks + budget breaches (18.B5). Budgets live in
 * `diagnostics/config.ts`'s `PERF_BUDGET_MS` — nothing here hard-codes a number. */
const stats = ref<PerfStat[]>([]);
const longTasks = ref<LogEntry[]>([]);
const loading = ref(true);

onMounted(async () => {
  const entries = await loadChannel('perf', 14);
  stats.value = computePerfStats(entries);
  longTasks.value = slowestLongTasks(entries);
  loading.value = false;
});

const breachCount = computed(() => stats.value.reduce((a, s) => a + s.breaches, 0));

const columns: Column<PerfStat>[] = [
  { key: 'source', label: 'المصدر', sortable: true },
  { key: 'kind', label: 'النوع', sortable: true },
  { key: 'count', label: 'العدد', numeric: true, sortable: true },
  { key: 'p50', label: 'p50 (ms)', numeric: true, sortable: true },
  { key: 'p95', label: 'p95 (ms)', numeric: true, sortable: true },
  { key: 'max', label: 'الأقصى (ms)', numeric: true, sortable: true },
  { key: 'breaches', label: 'تجاوزات الميزانية', numeric: true, sortable: true },
];

const KIND_LABEL: Record<PerfStat['kind'], string> = {
  service: 'خدمة', route: 'مسار', longtask: 'مهمة طويلة', startup: 'الإقلاع', snapshot: 'لقطة',
};
</script>

<template>
  <div class="space-y-4">
    <AppCard v-if="breachCount > 0" padding="sm" class="border-warning/40 bg-warning/5">
      <p class="flex items-center gap-2 text-body"><Gauge class="size-4 text-warning" /> {{ breachCount }} تجاوز لميزانية الأداء خلال آخر 14 يوماً</p>
    </AppCard>

    <DataTable :columns="columns" :rows="stats" :loading="loading" :empty-icon="Gauge" empty-title="لا توجد بيانات أداء بعد">
      <template #cell-kind="{ row }">{{ KIND_LABEL[row.kind] }}</template>
      <template #cell-p50="{ row }"><span class="num">{{ row.p50.toFixed(0) }}</span></template>
      <template #cell-p95="{ row }"><span class="num">{{ row.p95.toFixed(0) }}</span></template>
      <template #cell-max="{ row }"><span class="num">{{ row.max.toFixed(0) }}</span></template>
      <template #cell-breaches="{ row }">
        <StatusBadge v-if="row.breaches > 0" tone="warning" :label="String(row.breaches)" />
        <span v-else class="num text-text-secondary">0</span>
      </template>
    </DataTable>

    <AppCard v-if="longTasks.length" title="أبطأ المهام الطويلة" padding="sm">
      <ul class="divide-y divide-border text-xs">
        <li v-for="(t, i) in longTasks" :key="i" class="flex items-center justify-between py-1.5">
          <span class="text-text-secondary">{{ t.msg }}</span>
          <span class="num">{{ (t.data as any)?.durationMs?.toFixed(0) }} ms</span>
        </li>
      </ul>
    </AppCard>
  </div>
</template>

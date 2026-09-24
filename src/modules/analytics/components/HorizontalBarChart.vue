<script setup lang="ts">
import { computed } from 'vue';
import { formatMoney, formatNumber } from '@/modules/core/helpers/format';

/**
 * Simple horizontal bar list (no charting library) — used across the analytics tabs for
 * weekday mix, payment-method mix, top/bottom products, top customers. One hue, matching the
 * dataviz convention `SalesTrendChart.vue` already established for the home's chart.
 */
const props = withDefaults(
  defineProps<{ data: { label: string; value: number; sub?: string }[]; money?: boolean; tone?: 'primary' | 'danger' }>(),
  { tone: 'primary' },
);

const max = computed(() => Math.max(1, ...props.data.map((d) => Math.abs(d.value))));

function fmt(v: number) {
  return props.money ? formatMoney(v) : formatNumber(v);
}
</script>

<template>
  <div class="space-y-2.5">
    <div v-for="d in data" :key="d.label" class="flex items-center gap-3">
      <span class="w-24 shrink-0 truncate text-xs text-text-secondary" :title="d.label">{{ d.label }}</span>
      <div class="relative h-5 flex-1 overflow-hidden rounded bg-surface-hover">
        <div
          class="h-full rounded"
          :class="tone === 'danger' ? 'bg-danger' : 'bg-primary'"
          :style="{ width: `${(Math.abs(d.value) / max) * 100}%`, opacity: 0.75 }"
        />
      </div>
      <span class="num w-24 shrink-0 text-end text-xs font-medium">
        {{ fmt(d.value) }}
        <span v-if="d.sub" class="block text-tiny font-normal text-text-secondary">{{ d.sub }}</span>
      </span>
    </div>
    <p v-if="!data.length" class="py-6 text-center text-xs text-text-secondary">لا توجد بيانات كافية بعد</p>
  </div>
</template>

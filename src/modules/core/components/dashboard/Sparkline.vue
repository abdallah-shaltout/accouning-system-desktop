<script setup lang="ts">
import { computed } from 'vue';

/** Tiny inline sparkline (no library) for a KPI card — last N values, single hue, no axes. */
const props = withDefaults(defineProps<{ data: number[]; tone?: 'primary' | 'success' | 'danger' }>(), { tone: 'primary' });

const W = 96;
const H = 28;

const path = computed(() => {
  const values = props.data.length ? props.data : [0];
  const max = Math.max(...values, 0.0001);
  const min = Math.min(...values, 0);
  const range = max - min || 1;
  const step = W / Math.max(1, values.length - 1);
  return values.map((v, i) => `${i === 0 ? 'M' : 'L'}${round(i * step)},${round(H - ((v - min) / range) * H)}`).join(' ');
});

function round(n: number) {
  return Math.round(n * 10) / 10;
}

const colorVar = computed(() => (props.tone === 'success' ? 'var(--color-success)' : props.tone === 'danger' ? 'var(--color-danger)' : 'var(--color-primary)'));
</script>

<template>
  <svg :viewBox="`0 0 ${W} ${H}`" class="block h-6 w-24 shrink-0" preserveAspectRatio="none" aria-hidden="true">
    <path :d="path" fill="none" :stroke="colorVar" stroke-width="1.75" stroke-linejoin="round" stroke-linecap="round" opacity="0.8" />
  </svg>
</template>

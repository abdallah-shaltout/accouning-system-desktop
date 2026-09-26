<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — a small row of KPI cards (`{ label, value, trend?, to? }`) used at the top of
 * list/detail/dashboard pages. `trend` is a plain +/-percentage string, not mirrored in RTL
 * (CLAUDE.md RTL rule 17 — physical/numeric directions like trends are not mirrored). Optional `to`
 * makes the whole card a `RouterLink`.
 */
import { RouterLink, type RouteLocationRaw } from 'vue-router';

export interface StatCard {
  label: string;
  value: string;
  /** e.g. "+12%" / "-3%" — rendered LTR, colored by sign, never mirrored. */
  trend?: string;
  to?: RouteLocationRaw;
}

defineProps<{ cards: StatCard[] }>();

function trendTone(trend?: string): string {
  if (!trend) return 'text-text-secondary';
  return trend.trim().startsWith('-') ? 'text-danger' : 'text-success';
}
</script>

<template>
  <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
    <component
      :is="card.to ? RouterLink : 'div'"
      v-for="card in cards"
      :key="card.label"
      :to="card.to"
      class="rounded-xl border border-border bg-surface p-4 transition-colors"
      :class="card.to && 'hover:bg-surface-hover'"
    >
      <p class="text-xs text-text-secondary">{{ card.label }}</p>
      <div class="mt-1 flex items-baseline gap-2">
        <p class="num text-lg font-semibold">{{ card.value }}</p>
        <span v-if="card.trend" class="num text-xs" dir="ltr" :class="trendTone(card.trend)">{{ card.trend }}</span>
      </div>
    </component>
  </div>
</template>

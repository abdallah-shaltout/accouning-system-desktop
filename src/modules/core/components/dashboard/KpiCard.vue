<script setup lang="ts">
import type { Component } from 'vue';
import type { RouteLocationRaw } from 'vue-router';
import { TrendingDown, TrendingUp } from '@lucide/vue';
import { dirIcon } from '../../helpers/dirIcon';
import { formatNumber } from '../../helpers/format';
import DirIcon from '../ui/DirIcon.vue';

/** Stat tile: label · value · optional context line. Big values use proportional digits.
 *  v2 phase 10 (docs/v2/11 Part B.3): optional `changePct` (vs the previous period) + a `#spark`
 *  slot for a tiny sparkline, used by the new home's 4 KPIs. */
defineProps<{
  label: string;
  icon: Component;
  loading?: boolean;
  to?: RouteLocationRaw;
  tone?: 'danger' | 'warning';
  changePct?: number | null;
}>();
</script>

<template>
  <component
    :is="to ? 'RouterLink' : 'div'"
    :to="to"
    class="group flex flex-col rounded-xl border border-border bg-surface p-4 transition-colors"
    :class="to && 'hover:border-text-secondary/40'"
  >
    <div class="mb-3 flex items-center justify-between text-text-secondary">
      <span class="flex items-center gap-2 text-body">
        <component :is="icon" class="size-4" :stroke-width="1.75" />
        {{ label }}
      </span>
      <DirIcon v-if="to" :icon="dirIcon.open" class="size-4 opacity-0 transition-opacity group-hover:opacity-100" />
    </div>
    <div v-if="loading" class="h-8 w-2/3 animate-shimmer rounded-md bg-surface-hover" />
    <div v-else class="text-stat font-semibold leading-tight tracking-tight" :class="tone === 'danger' ? 'text-danger' : tone === 'warning' ? 'text-warning' : ''">
      <slot />
    </div>
    <div class="mt-1.5 min-h-4 text-xs text-text-secondary">
      <slot v-if="!loading" name="hint" />
    </div>
    <div v-if="!loading && (changePct !== undefined || $slots.spark)" class="mt-2 flex items-center justify-between gap-2 border-t border-border pt-2">
      <span v-if="changePct !== undefined && changePct !== null" class="num flex shrink-0 items-center gap-1 text-xs font-medium" :class="changePct >= 0 ? 'text-success' : 'text-danger'">
        <component :is="changePct >= 0 ? TrendingUp : TrendingDown" class="size-3" :stroke-width="2" />
        {{ formatNumber(Math.abs(changePct), 0) }}% <span class="font-normal text-text-secondary">عن الفترة السابقة</span>
      </span>
      <span v-else class="text-xs text-text-secondary">—</span>
      <div v-if="$slots.spark" class="shrink-0"><slot name="spark" /></div>
    </div>
  </component>
</template>

<script setup lang="ts">
import type { Component } from 'vue';
import type { RouteLocationRaw } from 'vue-router';
import { ChevronLeft } from '@lucide/vue';

/** Stat tile: label · value · optional context line. Big values use proportional digits. */
defineProps<{ label: string; icon: Component; loading?: boolean; to?: RouteLocationRaw; tone?: 'danger' | 'warning' }>();
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
      <ChevronLeft v-if="to" class="size-4 opacity-0 transition-opacity group-hover:opacity-100" />
    </div>
    <div v-if="loading" class="h-8 w-2/3 animate-shimmer rounded-md bg-surface-hover" />
    <div v-else class="text-stat font-semibold leading-tight tracking-tight" :class="tone === 'danger' ? 'text-danger' : tone === 'warning' ? 'text-warning' : ''">
      <slot />
    </div>
    <div class="mt-1.5 min-h-4 text-xs text-text-secondary">
      <slot v-if="!loading" name="hint" />
    </div>
  </component>
</template>

<script setup lang="ts">
import type { Insight } from '@/modules/core/services/insightTypes';

/**
 * Small contextual insight chip (docs/v2/11 D1 "inline hints"), e.g. on a product detail page:
 * "هذا المنتج منخفض المخزون". Driven by the same insight engine/rules as the home panel — no
 * separate hardcoded logic, just a different (compact) rendering of the same `Insight`.
 */
defineProps<{ insight: Insight }>();

const SEVERITY_CLASS: Record<Insight['severity'], string> = {
  critical: 'border-danger/30 bg-danger/10 text-danger',
  warning: 'border-warning/30 bg-warning/10 text-warning',
  info: 'border-primary/30 bg-primary/10 text-primary',
  positive: 'border-success/30 bg-success/10 text-success',
};
</script>

<template>
  <RouterLink
    :to="insight.actionTo"
    class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium transition-opacity hover:opacity-80"
    :class="SEVERITY_CLASS[insight.severity]"
  >
    <component :is="insight.icon" class="size-3.5" :stroke-width="2" />
    {{ insight.message }}
  </RouterLink>
</template>

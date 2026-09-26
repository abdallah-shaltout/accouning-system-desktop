<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — the shared top of a detail page: title, document number, `StatusBadge`, a
 * row of meta chips (date, party, branch…) and action buttons (print/edit/more). Built on
 * `PageHeader` for the back-link/title layout every page already uses, plus the number/status/chips
 * row this block adds.
 */
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import type { Tone } from '@/modules/core/helpers/labels';
import type { AppRoute } from '@/modules/core/types/route';

export interface MetaChip {
  label: string;
  value: string;
}

defineProps<{
  title: string;
  /** Document number/code, rendered LTR next to the title (e.g. "INV-2026-0042"). */
  number?: string;
  status?: { label: string; tone: Tone };
  chips?: MetaChip[];
  back?: AppRoute;
}>();
</script>

<template>
  <div>
    <PageHeader :title="title" :back="back">
      <template #badge>
        <span v-if="number" class="num text-body font-normal text-text-secondary">{{ number }}</span>
        <StatusBadge v-if="status" :label="status.label" :tone="status.tone" />
      </template>
      <template #actions><slot name="actions" /></template>
    </PageHeader>
    <div v-if="chips?.length" class="no-print -mt-3 mb-4 flex flex-wrap gap-x-5 gap-y-1 text-xs text-text-secondary">
      <span v-for="c in chips" :key="c.label"><span>{{ c.label }}:</span> <span class="num font-medium text-text-primary">{{ c.value }}</span></span>
    </div>
  </div>
</template>

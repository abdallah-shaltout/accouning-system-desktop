<script setup lang="ts">
import type { WizardState } from '../../types';
import { BUSINESS_TYPES } from '../../types';

const props = defineProps<{ state: WizardState }>();
</script>

<template>
  <div class="grid gap-3 sm:grid-cols-2">
    <button
      v-for="t in BUSINESS_TYPES"
      :key="t.value"
      type="button"
      class="rounded-xl border p-4 text-start transition-colors"
      :class="props.state.businessType === t.value ? 'border-primary bg-primary/5' : 'border-border bg-surface hover:bg-surface-hover'"
      @click="props.state.businessType = t.value"
    >
      <p class="text-sm font-semibold">{{ t.label }}</p>
      <p class="mt-1 text-xs leading-relaxed text-text-secondary">{{ t.note }}</p>
    </button>
  </div>
</template>

<script setup lang="ts">
/** Rebuilt on shadcn's Card (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props/slots as before. */
import { Card } from '@/modules/core/components/shadcn/card';

withDefaults(defineProps<{ title?: string; subtitle?: string; padding?: 'none' | 'sm' | 'md' }>(), { padding: 'md' });
</script>

<template>
  <Card class="gap-0 rounded-xl border-border bg-surface py-0 shadow-none">
    <header v-if="title || $slots.actions" class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
      <div class="min-w-0">
        <h2 class="truncate text-body font-semibold">{{ title }}</h2>
        <p v-if="subtitle" class="truncate text-xs text-text-secondary">{{ subtitle }}</p>
      </div>
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-1.5"><slot name="actions" /></div>
    </header>
    <div :class="{ 'p-6': padding === 'md', 'p-4': padding === 'sm' }">
      <slot />
    </div>
  </Card>
</template>

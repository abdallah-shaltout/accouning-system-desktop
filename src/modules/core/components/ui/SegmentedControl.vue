<script setup lang="ts" generic="V extends string | number">
/** Rebuilt on shadcn's ToggleGroup, i.e. reka-ui's ToggleGroupRoot (docs/v2/16-equal-rebrand-and-
 * ui-kit.md Phase C) — same props/model as before. Real upgrade over the hand-rolled button loop:
 * `type="single"` gives roving-tabindex arrow-key navigation between options for free. */
import type { Component } from 'vue';
import { ToggleGroup, ToggleGroupItem } from '@/modules/core/components/shadcn/toggle-group';

defineProps<{ options: { value: V; label: string; icon?: Component; count?: number }[]; size?: 'sm' | 'md' }>();
const model = defineModel<V>();
</script>

<template>
  <ToggleGroup
    :model-value="model as string | number | undefined"
    type="single"
    class="w-fit gap-0.5 rounded-lg border border-border bg-surface p-0.5"
    @update:model-value="(v) => { if (v !== undefined) model = v as V; }"
  >
    <ToggleGroupItem
      v-for="o in options"
      :key="String(o.value)"
      :value="o.value as string | number"
      class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-md bg-transparent font-medium data-[state=on]:bg-background data-[state=on]:text-text-primary data-[state=on]:shadow-sm data-[state=off]:text-text-secondary hover:data-[state=off]:text-text-primary hover:bg-transparent"
      :class="size === 'sm' ? 'h-6 px-2 text-xs' : 'h-7 px-3 text-body'"
    >
      <component :is="o.icon" v-if="o.icon" class="size-3.5" />
      {{ o.label }}
      <span v-if="o.count !== undefined" class="num rounded-full bg-surface-hover px-1.5 text-caption leading-4 text-text-secondary">{{ o.count }}</span>
    </ToggleGroupItem>
  </ToggleGroup>
</template>

<script setup lang="ts">
/** Rebuilt on shadcn's Empty primitives (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same
 * props/slots as before. Icon tile size/spacing stay our own values (not shadcn's emptyMediaVariants
 * "icon" size) since they're relied on visually across every empty table/list in the app. */
import type { Component } from 'vue';
import { Inbox } from '@lucide/vue';
import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent } from '@/modules/core/components/shadcn/empty';

withDefaults(defineProps<{ title?: string; description?: string; icon?: Component; compact?: boolean }>(), {
  title: 'لا توجد بيانات',
});
</script>

<template>
  <Empty class="border-none p-0" :class="compact ? 'py-8' : 'py-16'">
    <EmptyHeader class="gap-0">
      <EmptyMedia class="mb-3 size-11 rounded-xl border border-border bg-surface text-text-secondary">
        <component :is="icon ?? Inbox" class="size-5" :stroke-width="1.5" />
      </EmptyMedia>
      <EmptyTitle class="text-body font-medium">{{ title }}</EmptyTitle>
      <EmptyDescription v-if="description" class="mt-1 max-w-sm text-xs leading-5 text-text-secondary">{{ description }}</EmptyDescription>
    </EmptyHeader>
    <EmptyContent v-if="$slots.default" class="mt-4 flex-row gap-2">
      <slot />
    </EmptyContent>
  </Empty>
</template>

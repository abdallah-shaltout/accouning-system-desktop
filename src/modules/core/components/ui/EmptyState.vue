<script setup lang="ts">
import type { Component } from 'vue';
import { Inbox } from '@lucide/vue';

withDefaults(defineProps<{ title?: string; description?: string; icon?: Component; compact?: boolean }>(), {
  title: 'لا توجد بيانات',
});
</script>

<template>
  <div class="flex flex-col items-center justify-center text-center" :class="compact ? 'py-8' : 'py-16'">
    <div class="mb-3 flex size-11 items-center justify-center rounded-xl border border-border bg-surface text-text-secondary">
      <component :is="icon ?? Inbox" class="size-5" :stroke-width="1.5" />
    </div>
    <p class="text-body font-medium">{{ title }}</p>
    <p v-if="description" class="mt-1 max-w-sm text-xs leading-5 text-text-secondary">{{ description }}</p>
    <div v-if="$slots.default" class="mt-4 flex gap-2">
      <slot />
    </div>
  </div>
</template>

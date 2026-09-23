<script setup lang="ts" generic="V extends string | number">
import type { Component } from 'vue';

defineProps<{ options: { value: V; label: string; icon?: Component; count?: number }[]; size?: 'sm' | 'md' }>();
const model = defineModel<V>();
</script>

<template>
  <div role="tablist" class="inline-flex items-center gap-0.5 rounded-lg border border-border bg-surface p-0.5">
    <button
      v-for="o in options"
      :key="String(o.value)"
      type="button"
      role="tab"
      :aria-selected="model === o.value"
      class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-md font-medium transition-colors"
      :class="[
        size === 'sm' ? 'h-6 px-2 text-xs' : 'h-7 px-3 text-body',
        model === o.value ? 'bg-background text-text-primary shadow-sm' : 'text-text-secondary hover:text-text-primary',
      ]"
      @click="model = o.value"
    >
      <component :is="o.icon" v-if="o.icon" class="size-3.5" />
      {{ o.label }}
      <span v-if="o.count !== undefined" class="num rounded-full bg-surface-hover px-1.5 text-caption leading-4 text-text-secondary">{{ o.count }}</span>
    </button>
  </div>
</template>

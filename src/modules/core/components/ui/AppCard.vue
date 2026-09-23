<script setup lang="ts">
withDefaults(defineProps<{ title?: string; subtitle?: string; padding?: 'none' | 'sm' | 'md' }>(), { padding: 'md' });
</script>

<template>
  <section class="rounded-xl border border-border bg-surface">
    <header v-if="title || $slots.actions" class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
      <div class="min-w-0">
        <h2 class="truncate text-[13px] font-semibold">{{ title }}</h2>
        <p v-if="subtitle" class="truncate text-xs text-text-secondary">{{ subtitle }}</p>
      </div>
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-1.5"><slot name="actions" /></div>
    </header>
    <div :class="{ 'p-6': padding === 'md', 'p-4': padding === 'sm' }">
      <slot />
    </div>
  </section>
</template>

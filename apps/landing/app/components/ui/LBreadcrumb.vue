<script setup lang="ts">
import { ChevronLeft } from 'lucide-vue-next'

export interface Crumb { label: string; to?: string }
defineProps<{ items: Crumb[] }>()
</script>

<template>
  <nav aria-label="مسار التصفح">
    <ol class="flex flex-wrap items-center gap-1.5 text-sm text-ink-mute">
      <li v-for="(c, i) in items" :key="c.label" class="flex items-center gap-1.5">
        <NuxtLink v-if="c.to" :to="c.to" class="transition-colors hover:text-coral-500">{{ c.label }}</NuxtLink>
        <span v-else aria-current="page" class="text-ink-soft">{{ c.label }}</span>
        <ChevronLeft v-if="i < items.length - 1" class="size-3.5" aria-hidden="true" />
      </li>
    </ol>
  </nav>
</template>

<script setup lang="ts">
/**
 * v2 doc 17 Phase F-0 — replaces the ad-hoc `<AppCard>` + heading per form part with one shared
 * section block: title/description header, optional collapse, and a responsive 1/2/3-column grid
 * for its fields (`FormField`s go in the default slot). Built on `AppCard` so it keeps the same
 * surface/border/radius as every other card on the page.
 */
import { ref } from 'vue';
import { ChevronDown } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';

const props = withDefaults(
  defineProps<{
    title?: string;
    description?: string;
    /** 1 = single column (default), 2/3 = responsive grid that collapses to 1 column below `sm`. */
    columns?: 1 | 2 | 3;
    collapsible?: boolean;
    defaultOpen?: boolean;
  }>(),
  { columns: 1, defaultOpen: true },
);

const open = ref(props.defaultOpen);
const gridClass = { 1: '', 2: 'sm:grid-cols-2', 3: 'sm:grid-cols-3' } as const;
</script>

<template>
  <AppCard :title="collapsible ? undefined : title" :subtitle="collapsible ? undefined : description" padding="sm">
    <button
      v-if="collapsible"
      type="button"
      class="mb-3 flex w-full items-center justify-between gap-2 text-start"
      :aria-expanded="open"
      @click="open = !open"
    >
      <span>
        <span class="text-body font-semibold">{{ title }}</span>
        <span v-if="description" class="mt-0.5 block text-xs font-normal text-text-secondary">{{ description }}</span>
      </span>
      <ChevronDown class="size-4 shrink-0 text-text-secondary transition-transform" :class="!open && '-rotate-90'" />
    </button>
    <div v-if="!collapsible || open" class="grid grid-cols-1 gap-4" :class="gridClass[columns]">
      <slot />
    </div>
  </AppCard>
</template>

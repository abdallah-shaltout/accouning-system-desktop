<script setup lang="ts">
import type { Component } from 'vue'
import { ArrowDown, ArrowUp } from 'lucide-vue-next'

/**
 * Wide single-open accordion bar (reference: the review rows — light bar with a dark square
 * arrow when closed, a tall card with the content indented when open). Shared by the business
 * types and the FAQ.
 */
const props = defineProps<{ id: string; title: string; icon?: Component; open: boolean; headingLevel?: 'h3' | 'h4' }>()
defineEmits<{ toggle: [] }>()
const heading = computed(() => props.headingLevel ?? 'h3')
</script>

<template>
  <div class="rounded-card bg-panel-50 transition-colors duration-500" :class="open ? '' : 'hover:bg-panel-100'">
    <component :is="heading" class="m-0">
      <button
        :id="`${id}-btn`"
        type="button"
        class="flex w-full items-center justify-between gap-4 px-5 text-start transition-[padding] duration-700 ease-out-expo sm:px-7"
        :class="open ? 'pt-8 pb-2 lg:ps-[23%] lg:pt-11' : 'py-7'"
        :aria-expanded="open"
        :aria-controls="`${id}-panel`"
        @click="$emit('toggle')"
      >
        <span class="flex items-center gap-3 font-display text-ink" :class="open ? 'text-2xl' : 'text-xl'">
          <component :is="icon" v-if="icon" class="size-6 shrink-0" :stroke-width="1.75" />
          {{ title }}
        </span>
        <span
          class="grid size-8 shrink-0 place-items-center rounded-lg transition-colors duration-500"
          :class="open ? 'bg-white text-ink shadow-soft lg:-mt-10' : 'bg-ink text-white'"
          aria-hidden="true"
        >
          <ArrowUp v-if="open" class="size-4" />
          <ArrowDown v-else class="size-4" />
        </span>
      </button>
    </component>
    <div
      :id="`${id}-panel`"
      role="region"
      :aria-labelledby="`${id}-btn`"
      class="grid transition-[grid-template-rows,opacity] duration-700 ease-out-expo"
      :class="open ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'"
    >
      <div class="overflow-hidden">
        <div class="px-5 pt-5 pb-10 sm:px-7 lg:ps-[23%] lg:pb-16">
          <slot />
        </div>
      </div>
    </div>
  </div>
</template>

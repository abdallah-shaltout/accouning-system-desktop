<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import { cn } from '@/modules/core/helpers/utils'

const props = withDefaults(defineProps<{
  class?: HTMLAttributes["class"]
  /** Root element tag. AppCard.vue uses "section" to keep its original semantic markup. */
  as?: string
}>(), { as: "div" })
</script>

<template>
  <component
    :is="as"
    data-slot="card"
    :class="
      cn(
        'bg-card text-card-foreground flex flex-col gap-6 rounded-xl border py-6 shadow-sm',
        props.class,
      )
    "
  >
    <slot />
  </component>
</template>

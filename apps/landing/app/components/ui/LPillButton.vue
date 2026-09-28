<script setup lang="ts">
import { NuxtLink } from '#components'

const props = withDefaults(
  defineProps<{
    to?: string
    tone?: 'coral' | 'white' | 'ghost' | 'ink'
    size?: 'sm' | 'md' | 'lg'
    type?: 'button' | 'submit'
  }>(),
  { tone: 'coral', size: 'md', type: 'button' },
)

const isExternal = computed(() => !!props.to && /^(https?:|mailto:|tel:)/.test(props.to))
</script>

<template>
  <component
    :is="to ? (isExternal ? 'a' : NuxtLink) : 'button'"
    :to="to && !isExternal ? to : undefined"
    :href="to && isExternal ? to : undefined"
    :type="to ? undefined : type"
    class="group/btn relative inline-flex shrink-0 select-none items-center justify-center gap-2 rounded-full font-display font-normal whitespace-nowrap transition-[background-color,color,transform,box-shadow] duration-300 ease-out-expo hover:-translate-y-px active:scale-[0.98]"
    :class="[
      {
        'h-9 px-5 text-sm': size === 'sm',
        'h-12 px-7 text-base': size === 'md',
        'h-14 px-8 text-lg': size === 'lg',
      },
      {
        'bg-coral-500 text-white shadow-[inset_0_1px_0_rgb(255_255_255/0.25)] hover:bg-coral-600': tone === 'coral',
        'bg-white text-ink hover:bg-cream': tone === 'white',
        'border border-current/25 bg-transparent hover:bg-current/5': tone === 'ghost',
        'bg-ink text-white hover:bg-forest-800': tone === 'ink',
      },
    ]"
  >
    <slot />
  </component>
</template>

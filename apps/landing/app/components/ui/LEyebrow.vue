<script setup lang="ts">
withDefaults(defineProps<{ tone?: 'coral' | 'white' | 'ink'; dot?: boolean }>(), { tone: 'coral', dot: false })
</script>

<template>
  <p
    class="inline-flex items-center gap-2 text-eyebrow font-medium"
    :class="{
      'text-coral-500': tone === 'coral',
      'text-white/85': tone === 'white',
      'text-ink': tone === 'ink',
    }"
    data-reveal="eyebrow"
  >
    <svg v-if="dot" class="size-3.5 shrink-0" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.3" opacity=".45" />
      <circle cx="8" cy="8" r="3.4" fill="currentColor" />
    </svg>
    <slot />
  </p>
</template>

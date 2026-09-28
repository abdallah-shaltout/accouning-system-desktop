<script setup lang="ts">
withDefaults(defineProps<{ wordmark?: boolean; tone?: 'coral' | 'white' | 'ink' }>(), { wordmark: false, tone: 'coral' })
</script>

<template>
  <span class="inline-flex items-center gap-2.5" :class="{ 'text-coral-500': tone === 'coral', 'text-white': tone === 'white', 'text-ink': tone === 'ink' }">
    <!-- The "=" mark: two offset rounded bars — Equal. -->
    <svg class="h-[1.4em] w-auto shrink-0" viewBox="0 0 36 28" aria-hidden="true">
      <rect x="0" y="2" width="30" height="9" rx="4.5" fill="currentColor" />
      <rect x="6" y="17" width="30" height="9" rx="4.5" fill="currentColor" />
    </svg>
    <span v-if="wordmark" class="font-display text-[1.35em] leading-none font-medium" :class="tone === 'coral' ? 'text-ink' : ''">
      {{ APP_SHORT_AR }}
    </span>
  </span>
</template>

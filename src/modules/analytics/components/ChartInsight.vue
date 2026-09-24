<script setup lang="ts">
import { Lightbulb } from '@lucide/vue';

/** One-sentence, plain-Arabic auto-generated insight shown under a chart's title (docs/v2/11 Part C). */
defineProps<{ text: string }>();
</script>

<template>
  <p class="mb-3 flex items-start gap-1.5 text-xs text-text-secondary">
    <Lightbulb class="mt-0.5 size-3.5 shrink-0 text-primary" :stroke-width="1.75" />
    <span>{{ text }}</span>
  </p>
</template>

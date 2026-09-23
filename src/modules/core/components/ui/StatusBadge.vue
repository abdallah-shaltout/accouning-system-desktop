<script setup lang="ts">
import type { Tone } from '../../helpers/labels';

withDefaults(defineProps<{ tone?: Tone; label: string; dot?: boolean }>(), { tone: 'neutral', dot: true });

const toneText: Record<Tone, string> = {
  neutral: 'text-text-secondary',
  success: 'text-success',
  warning: 'text-warning',
  danger: 'text-danger',
  primary: 'text-primary',
};
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full bg-surface-hover px-2 py-0.5 text-xs font-medium"
    :class="toneText[tone]"
  >
    <span v-if="dot" class="size-1.5 rounded-full bg-current" />
    {{ label }}
  </span>
</template>

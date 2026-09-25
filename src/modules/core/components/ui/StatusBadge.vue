<script setup lang="ts">
/** Rebuilt on shadcn's Badge (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props as
 * before. Uses the "secondary" shadcn variant as a neutral pill base for every tone, since our
 * tones are distinguished by text/dot color, not by shadcn's own variant palette. */
import { Badge } from '@/modules/core/components/shadcn/badge';
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
  <Badge variant="secondary" class="gap-1.5 whitespace-nowrap rounded-full bg-surface-hover px-2 py-0.5 text-xs font-medium" :class="toneText[tone]">
    <span v-if="dot" class="size-1.5 rounded-full bg-current" />
    {{ label }}
  </Badge>
</template>

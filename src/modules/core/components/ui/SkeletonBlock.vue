<script setup lang="ts">
/** Rebuilt on shadcn's Skeleton (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same props as
 * before. Keeps our own shimmer animation + surface-hover fill rather than shadcn's pulse/primary-10
 * default, since that's a more noticeable loading cue against our light/dark surfaces. */
import { Skeleton } from '@/modules/core/components/shadcn/skeleton';

withDefaults(defineProps<{ lines?: number; height?: string }>(), { lines: 1, height: 'h-4' });
</script>

<template>
  <div class="space-y-2" aria-hidden="true">
    <Skeleton
      v-for="i in lines"
      :key="i"
      class="animate-shimmer rounded-md bg-surface-hover"
      :class="height"
      :style="{ width: lines > 1 && i === lines ? '60%' : '100%' }"
    />
  </div>
</template>

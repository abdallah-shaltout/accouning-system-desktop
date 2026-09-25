<script setup lang="ts">
/** Rebuilt on shadcn's Empty primitives (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — same
 * props/emits as before. */
import { RotateCcw, TriangleAlert } from '@lucide/vue';
import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent } from '@/modules/core/components/shadcn/empty';
import AppButton from './AppButton.vue';

withDefaults(defineProps<{ title?: string; message?: string | null; compact?: boolean }>(), {
  title: 'تعذر تحميل البيانات',
});
defineEmits<{ retry: [] }>();
</script>

<template>
  <Empty class="border-none p-0" :class="compact ? 'py-8' : 'py-16'" role="alert">
    <EmptyHeader class="gap-0">
      <EmptyMedia class="mb-3 size-11 rounded-xl border border-danger/30 bg-danger/10 text-danger">
        <TriangleAlert class="size-5" :stroke-width="1.5" />
      </EmptyMedia>
      <EmptyTitle class="text-body font-medium">{{ title }}</EmptyTitle>
      <EmptyDescription v-if="message" class="mt-1 max-w-sm text-xs leading-5 text-text-secondary">{{ message }}</EmptyDescription>
    </EmptyHeader>
    <EmptyContent class="mt-4">
      <AppButton size="sm" :icon="RotateCcw" @click="$emit('retry')">إعادة المحاولة</AppButton>
    </EmptyContent>
  </Empty>
</template>

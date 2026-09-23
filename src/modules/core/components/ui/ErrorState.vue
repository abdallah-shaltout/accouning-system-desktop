<script setup lang="ts">
import { RotateCcw, TriangleAlert } from '@lucide/vue';
import AppButton from './AppButton.vue';

withDefaults(defineProps<{ title?: string; message?: string | null; compact?: boolean }>(), {
  title: 'تعذر تحميل البيانات',
});
defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="flex flex-col items-center justify-center text-center" :class="compact ? 'py-8' : 'py-16'" role="alert">
    <div class="mb-3 flex size-11 items-center justify-center rounded-xl border border-danger/30 bg-danger/10 text-danger">
      <TriangleAlert class="size-5" :stroke-width="1.5" />
    </div>
    <p class="text-body font-medium">{{ title }}</p>
    <p v-if="message" class="mt-1 max-w-sm text-xs leading-5 text-text-secondary">{{ message }}</p>
    <AppButton class="mt-4" size="sm" :icon="RotateCcw" @click="$emit('retry')">إعادة المحاولة</AppButton>
  </div>
</template>

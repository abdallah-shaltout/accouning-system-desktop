<script setup lang="ts">
withDefaults(defineProps<{ lines?: number; height?: string }>(), { lines: 1, height: 'h-4' });
</script>

<template>
  <div class="space-y-2" aria-hidden="true">
    <div
      v-for="i in lines"
      :key="i"
      class="animate-shimmer rounded-md bg-surface-hover"
      :class="height"
      :style="{ width: lines > 1 && i === lines ? '60%' : '100%' }"
    />
  </div>
</template>

<script setup lang="ts">
defineProps<{ label?: string; description?: string; disabled?: boolean }>();
const model = defineModel<boolean>({ default: false });
</script>

<template>
  <label class="flex cursor-pointer items-start gap-3" :class="disabled && 'cursor-not-allowed opacity-60'">
    <button
      type="button"
      role="switch"
      :aria-checked="model"
      :disabled="disabled"
      class="relative mt-0.5 inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors"
      :class="model ? 'border-primary bg-primary' : 'border-border bg-surface-hover'"
      @click="model = !model"
    >
      <span
        class="absolute size-3.5 rounded-full bg-white shadow-sm transition-all"
        :class="model ? 'start-[18px]' : 'start-0.5'"
      />
    </button>
    <span v-if="label || description" class="min-w-0">
      <span class="block text-body text-text-primary">{{ label }}</span>
      <span v-if="description" class="block text-xs text-text-secondary">{{ description }}</span>
    </span>
  </label>
</template>

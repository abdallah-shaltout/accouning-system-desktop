<script setup lang="ts" generic="V extends string | number">
import { useId } from 'vue';
import { ChevronDown } from '@lucide/vue';

defineProps<{
  label?: string;
  options: { value: V; label: string; disabled?: boolean }[];
  placeholder?: string;
  error?: string;
  hint?: string;
  required?: boolean;
  disabled?: boolean;
}>();

const model = defineModel<V | undefined | ''>();
const id = useId();
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <div class="relative">
      <select
        :id="id"
        v-model="model"
        :disabled="disabled"
        :aria-invalid="!!error || undefined"
        class="control appearance-none"
        :class="model === '' || model === undefined ? 'text-text-secondary' : ''"
      >
        <option v-if="placeholder !== undefined" value="">{{ placeholder }}</option>
        <option v-for="o in options" :key="String(o.value)" :value="o.value" :disabled="o.disabled" class="text-text-primary">
          {{ o.label }}
        </option>
      </select>
      <ChevronDown class="pointer-events-none absolute end-2.5 top-1/2 size-4 -translate-y-1/2 text-text-secondary" />
    </div>
    <p v-if="error" class="mt-1 text-xs text-danger">{{ error }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>

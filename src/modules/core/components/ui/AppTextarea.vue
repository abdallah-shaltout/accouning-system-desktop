<script setup lang="ts">
import { useId } from 'vue';

withDefaults(defineProps<{ label?: string; placeholder?: string; rows?: number; error?: string; disabled?: boolean }>(), { rows: 3 });
const model = defineModel<string | undefined>();
const id = useId();
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">{{ label }}</label>
    <textarea
      :id="id"
      v-model="model"
      :rows="rows"
      :placeholder="placeholder"
      :disabled="disabled"
      :aria-invalid="!!error || undefined"
      class="control"
    />
    <p v-if="error" class="mt-1 text-xs text-danger">{{ error }}</p>
  </div>
</template>
